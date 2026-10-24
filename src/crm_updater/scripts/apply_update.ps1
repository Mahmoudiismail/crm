
param(
    [string]$LogPath,
    [string]$DownloadsDir,
    [int]$ParentPid,
    [string]$ReplacementMapPath
)

$ErrorActionPreference = 'Stop'

function Write-Log {
    param([string]$Message)
    $Timestamp = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'
    $LogLine = "[$Timestamp] $Message"
    Write-Output $LogLine
    if ($LogPath) {
        Add-Content -LiteralPath $LogPath -Value $LogLine
    }
}

try {
    Write-Log "Detached update process started."
    Write-Log "Downloads directory resolved to: $DownloadsDir"
    Write-Log "Waiting for original updater process (PID: $ParentPid) to exit..."

    $TimeoutSeconds = 30
    $WaitCount = 0
    while ((Get-Process -Id $ParentPid -ErrorAction SilentlyContinue) -and ($WaitCount -lt $TimeoutSeconds)) {
        Start-Sleep -Seconds 1
        $WaitCount++
    }

    if (Get-Process -Id $ParentPid -ErrorAction SilentlyContinue) {
        Write-Log "FAILURE: Original updater process (PID: $ParentPid) failed to terminate after $TimeoutSeconds seconds."
        throw "Original updater termination timeout"
    } else {
        Write-Log "Original updater process terminated successfully."
    }

    if (-not (Test-Path -LiteralPath $ReplacementMapPath -PathType Leaf)) {
        throw "Replacement map file is missing."
    }
    $Config = Get-Content -LiteralPath $ReplacementMapPath -Raw | ConvertFrom-Json

    # Validate the entire replacement set before stopping any application.
    if (-not $Config.file_replacements -or @($Config.file_replacements).Count -eq 0) {
        throw "Update contains no file replacements."
    }
    foreach ($Item in $Config.file_replacements) {
        if ([string]::IsNullOrWhiteSpace($Item.source_path) -or
            [string]::IsNullOrWhiteSpace($Item.destination_path)) {
            throw "Update contains a replacement with an empty source or destination path."
        }
        if (-not (Test-Path -LiteralPath $Item.source_path -PathType Leaf)) {
            throw "Required update source file is missing: $($Item.source_path)"
        }
        $DestinationDirectory = Split-Path -Parent -LiteralPath $Item.destination_path
        if ([string]::IsNullOrWhiteSpace($DestinationDirectory) -or
            -not (Test-Path -LiteralPath $DestinationDirectory -PathType Container)) {
            throw "Destination directory does not exist: $DestinationDirectory"
        }
    }

    if ($Config.apps_to_stop) {
        foreach ($App in $Config.apps_to_stop) {
            $ProcessName = $App.process_name
            $TargetPaths = @($App.target_paths)

            $Processes = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue
            if ($Processes) {
                foreach ($Proc in $Processes) {
                    $ProcPath = $null
                    $PathError = $null

                    try {
                        $ProcPath = $Proc.Path
                    } catch {
                        $PathError = $_
                    }

                    if ([string]::IsNullOrWhiteSpace($ProcPath)) {
                        try {
                            $ProcPath = $Proc.MainModule.FileName
                        } catch {
                            if (-not $PathError) {
                                $PathError = $_
                            } else {
                                $PathError = "$PathError | $_"
                            }
                        }
                    }

                    if ([string]::IsNullOrWhiteSpace($ProcPath)) {
                        Write-Log "FAILURE: Cannot safely inspect process path for process ID $($Proc.Id) ($ProcessName). Error: $PathError"
                        throw "Unsafe process targeting: Cannot inspect process path."
                    }

                    $IsTargetMatch = $false
                    foreach ($tp in $TargetPaths) {
                        if ([string]::Equals($ProcPath.Trim(), $tp.Trim(), [System.StringComparison]::OrdinalIgnoreCase)) {
                            $IsTargetMatch = $true
                            break
                        }
                    }

                    if ($IsTargetMatch) {
                        Write-Log "Target process '$ProcessName' (PID: $($Proc.Id)) matches one of the target paths. Stopping..."

                        try {
                            Stop-Process -Id $Proc.Id -Force -ErrorAction Stop
                        } catch {
                            Write-Log "FAILURE: Stop-Process failed for process '$ProcessName' (PID: $($Proc.Id)). Error: $_"
                            throw "Process termination error"
                        }

                        $WaitCount = 0
                        while ((Get-Process -Id $Proc.Id -ErrorAction SilentlyContinue) -and ($WaitCount -lt $TimeoutSeconds)) {
                            Start-Sleep -Seconds 1
                            $WaitCount++
                        }

                        if (Get-Process -Id $Proc.Id -ErrorAction SilentlyContinue) {
                            Write-Log "FAILURE: Process '$ProcessName' (PID: $($Proc.Id)) failed to terminate after $TimeoutSeconds seconds."
                            throw "Process termination timeout"
                        } else {
                            Write-Log "Process '$ProcessName' (PID: $($Proc.Id)) terminated successfully."
                        }
                    } else {
                        Write-Log "Process '$ProcessName' (PID: $($Proc.Id)) is running at a different path ($ProcPath). Skipping termination."
                    }
                }
            } else {
                Write-Log "Target process '$ProcessName' is not running. No stop required."
            }
        }
    }

    # Stage every replacement beside its destination, verify it, then replace files.
    # Keep backups until every replacement has succeeded so a partial update can roll back.
    $AppliedReplacements = New-Object 'System.Collections.Generic.List[object]'
    $ReplacementPhaseComplete = $false

    if ($Config.file_replacements) {
        foreach ($Item in $Config.file_replacements) {
            $SrcPath = $Item.source_path
            $DstPath = $Item.destination_path
            $DestinationDirectory = Split-Path -Parent -LiteralPath $DstPath
            $FileName = Split-Path -Leaf -LiteralPath $DstPath
            $UniqueSuffix = [Guid]::NewGuid().ToString('N')
            $StagedPath = Join-Path $DestinationDirectory ".$FileName.update-$UniqueSuffix.tmp"
            $BackupPath = Join-Path $DestinationDirectory ".$FileName.update-$UniqueSuffix.bak"
            $HadOriginal = Test-Path -LiteralPath $DstPath -PathType Leaf
            $BackupCreated = $false

            try {
                Write-Log "Staging '$SrcPath' for '$DstPath'..."
                Copy-Item -LiteralPath $SrcPath -Destination $StagedPath -ErrorAction Stop
                $SrcHash = (Get-FileHash -LiteralPath $SrcPath -Algorithm SHA256).Hash
                $StagedHash = (Get-FileHash -LiteralPath $StagedPath -Algorithm SHA256).Hash
                if ($SrcHash -ne $StagedHash) {
                    throw "Staged file verification failed for '$DstPath'"
                }

                if ($HadOriginal) {
                    Move-Item -LiteralPath $DstPath -Destination $BackupPath -ErrorAction Stop
                    $BackupCreated = $true
                }

                # Record rollback information before moving the staged file into place.
                $AppliedReplacements.Add([PSCustomObject]@{
                    Destination = $DstPath
                    Backup = $BackupPath
                    HadOriginal = $HadOriginal
                })

                Move-Item -LiteralPath $StagedPath -Destination $DstPath -ErrorAction Stop
                $DestinationHash = (Get-FileHash -LiteralPath $DstPath -Algorithm SHA256).Hash
                if ($SrcHash -ne $DestinationHash) {
                    throw "Destination verification failed for '$DstPath'"
                }
                Write-Log "Successfully replaced '$DstPath' and verified SHA-256 hash."
            } catch {
                if ($BackupCreated -and (Test-Path -LiteralPath $BackupPath -PathType Leaf)) {
                    if (Test-Path -LiteralPath $DstPath) {
                        Remove-Item -LiteralPath $DstPath -Force -ErrorAction SilentlyContinue
                    }
                    Move-Item -LiteralPath $BackupPath -Destination $DstPath -ErrorAction SilentlyContinue
                }
                throw
            } finally {
                if (Test-Path -LiteralPath $StagedPath) {
                    Remove-Item -LiteralPath $StagedPath -Force -ErrorAction SilentlyContinue
                }
            }
        }
    }

    $ReplacementPhaseComplete = $true

    # All replacements verified; backups are no longer needed.
    foreach ($Applied in $AppliedReplacements) {
        if ($Applied.HadOriginal -and (Test-Path -LiteralPath $Applied.Backup -PathType Leaf)) {
            try {
                Remove-Item -LiteralPath $Applied.Backup -Force -ErrorAction Stop
            } catch {
                Write-Log "WARNING: Updated files are verified, but backup cleanup failed for '$($Applied.Backup)': $_"
            }
        }
    }

    if ($Config.restart_apps) {
        foreach ($App in $Config.restart_apps) {
            $DstPath = $App.destination_path
            $WorkDir = $App.working_directory
            $Autostart = $App.autostart
            $RestartArgs = @($App.restart_args)

            if ($Autostart) {
                if (Test-Path -LiteralPath $DstPath) {
                    Write-Log "Autostart is enabled. Starting '$DstPath'..."
                    if ($RestartArgs -and $RestartArgs.Count -gt 0) {
                        Start-Process -FilePath $DstPath -WorkingDirectory $WorkDir -ArgumentList $RestartArgs
                    } else {
                        Start-Process -FilePath $DstPath -WorkingDirectory $WorkDir
                    }
                    Write-Log "Started '$DstPath' successfully."
                }
            } else {
                Write-Log "Autostart is disabled for '$DstPath'. Leaving it stopped."
            }
        }
    }

    if ($Config.file_replacements) {
        foreach ($Item in $Config.file_replacements) {
            $SrcPath = $Item.source_path
            if (Test-Path -LiteralPath $SrcPath) {
                Remove-Item -LiteralPath $SrcPath -Force
                Write-Log "Cleaned up source file '$SrcPath'."
            }
        }
    }

    Write-Log "SUCCESS: Update completed successfully."
} catch {
    Write-Log "FAILURE: An error occurred during the update process: $_"

    # If the replacement phase did not finish, restore every previously replaced file.
    if (-not $ReplacementPhaseComplete -and $AppliedReplacements) {
        for ($Index = $AppliedReplacements.Count - 1; $Index -ge 0; $Index--) {
            $Applied = $AppliedReplacements[$Index]
            try {
                if ($Applied.HadOriginal -and (Test-Path -LiteralPath $Applied.Backup -PathType Leaf)) {
                    if (Test-Path -LiteralPath $Applied.Destination) {
                        Remove-Item -LiteralPath $Applied.Destination -Force -ErrorAction Stop
                    }
                    Move-Item -LiteralPath $Applied.Backup -Destination $Applied.Destination -ErrorAction Stop
                    Write-Log "Rollback restored '$($Applied.Destination)'."
                } elseif (-not $Applied.HadOriginal -and (Test-Path -LiteralPath $Applied.Destination)) {
                    Remove-Item -LiteralPath $Applied.Destination -Force -ErrorAction Stop
                    Write-Log "Rollback removed newly created '$($Applied.Destination)'."
                }
            } catch {
                Write-Log "ROLLBACK FAILURE for '$($Applied.Destination)': $_"
                $RollbackFailed = $true
            }
        }
    }

    $UpdateFailed = $true
} finally {
    if (Test-Path -LiteralPath $ReplacementMapPath) {
        Remove-Item -LiteralPath $ReplacementMapPath -Force -ErrorAction SilentlyContinue
    }
    if ($UpdateFailed) {
        exit 1
    }
}
