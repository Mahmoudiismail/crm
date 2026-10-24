# OLAP Cube Slicer Fix Plan

## Root Cause
When manipulating OLAP pivot slicers, assigning an array of strings to `$cache.VisibleSlicerItemsList` throws "The item could not be found in the OLAP Cube" if any of the items in the list do not actually exist in the cube, or if the array is empty. Furthermore, Excel requires at least one slicer item to remain visible at all times.

In `powershell.rs`, there are a few places where this could fail:
1. When filtering for `executive clinic` (line 253):
   `$visibleList = @(); foreach ($m in $monthItems) { $visibleList += $m.Name }; $monthSlicerCache.VisibleSlicerItemsList = $visibleList`
2. When filtering for `otherMonths` (line 361):
   `$monthSlicerCache.VisibleSlicerItemsList = $visibleList`
3. When filtering for `$currentMonthItem` (line 432):
   `$monthSlicerCache.VisibleSlicerItemsList = @($currentMonthItem.Name)`

The fix is to ensure the list is strictly valid and explicitly catch/ignore assignments that might throw due to missing backend OLAP cube data, OR verify that the assigned `Name` genuinely exists in the cube. Since we obtain `$m.Name` directly from `SlicerCacheLevels.Item(1).SlicerItems`, they *should* exist. However, if `$visibleList` ends up empty, Excel throws this error. Also, PowerShell's `$errorActionPreference = "Stop"` causes the entire script to abort if `VisibleSlicerItemsList` assignment fails for any reason.

## Changes
1. Modify `powershell.rs` inside `tasker/crm_open_sohail/powershell.rs`.
2. Wrap assignments to `$branchSlicerCache.VisibleSlicerItemsList` and `$monthSlicerCache.VisibleSlicerItemsList` in `try/catch` blocks so that a missing slicer configuration in the OLAP Cube doesn't fatally crash the entire pivot extraction. We will simply output a warning and `continue` to the next branch/month.
3. Validate that `$visibleList` has a count `> 0` before assigning.

Specifically:
```powershell
try {
    $branchSlicerCache.VisibleSlicerItemsList = @($bName)
} catch {
    Write-Output "Warning: Could not set branch OLAP slicer for $($bName) - $_"
    continue
}
```
And similarly for the month slicers.
