(async function(timeoutMinutes) {
                                            function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }
                                            let doc = document;
                                            let mainDoc = document;
                                            let logs = [];
                                            let xlsxOption = null;
                                            const deadlineXlsx = Date.now() + timeoutMinutes * 60 * 1000;
                                            while (Date.now() <= deadlineXlsx) {
                                                let listItems = doc.querySelectorAll('.dx-list-item-content');
                                                for (let item of listItems) {
                                                    if (item.textContent.trim() === 'XLSX') { xlsxOption = item.closest('.dx-list-item'); break; }
                                                }
                                                if (!xlsxOption) {
                                                    let allSpans = doc.querySelectorAll('span, div');
                                                    for (let span of allSpans) {
                                                        if (span.textContent.trim() === 'XLSX' && span.offsetParent !== null) {
                                                            xlsxOption = span; break;
                                                        }
                                                    }
                                                }

                                                if (!xlsxOption) {
                                                    listItems = mainDoc.querySelectorAll('.dx-list-item-content');
                                                    for (let item of listItems) {
                                                        if (item.textContent.trim() === 'XLSX') { xlsxOption = item.closest('.dx-list-item'); break; }
                                                    }
                                                }
                                                if (!xlsxOption) {
                                                    let allSpans = mainDoc.querySelectorAll('span, div');
                                                    for (let span of allSpans) {
                                                        if (span.textContent.trim() === 'XLSX' && span.offsetParent !== null) {
                                                            xlsxOption = span; break;
                                                        }
                                                    }
                                                }

                                                if (xlsxOption) break;
                                                await sleep(200);
                                            }

                                            if (!xlsxOption) return JSON.stringify({ status: "ERROR", msg: "XLSX option not found.", logs });
                                            xlsxOption.click();
                                            logs.push("Clicked XLSX option");
                                            return JSON.stringify({ status: "SUCCESS", logs });
                                        })(__TIMEOUT_MINUTES__);
