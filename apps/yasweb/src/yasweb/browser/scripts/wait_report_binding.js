(async function(reportName) {
                                            function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }
                                            let logs = [];
                                            let doc = document;
                                            let reportBound = false;
                                            logs.push("Waiting for report binding...");
                                            for (let i = 0; i < 30; i++) {
                                                let selects = doc.querySelectorAll('mat-select');
                                                for (let s of selects) {
                                                    if (s.innerText.includes(__REPORT_NAME_JSON__) || s.textContent.includes(__REPORT_NAME_JSON__)) {
                                                        reportBound = true; break;
                                                    }
                                                }
                                                if (!reportBound) {
                                                    let headers = doc.querySelectorAll('.fw-semibold, .fw-bold');
                                                    for (let h of headers) {
                                                        if (h.innerText.includes(__REPORT_NAME_JSON__) || h.textContent.includes(__REPORT_NAME_JSON__)) {
                                                            reportBound = true; break;
                                                        }
                                                    }
                                                }
                                                if (reportBound) break;
                                                await sleep(1000);
                                            }
                                            if (!reportBound) return JSON.stringify({ status: "ERROR", msg: "Binding timeout.", logs });
                                            return JSON.stringify({ status: "SUCCESS", logs });
                                        })(__REPORT_NAME_JSON__);
