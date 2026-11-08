(async function(timeoutMinutes) {
                                            function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }
                                            let logs = [];
                                            let doc = document;

                                            logs.push("Waiting for Search button to appear...");
                                            let clickedSearch = false;
                                            const deadlineSearch = Date.now() + timeoutMinutes * 60 * 1000;
                                            while (Date.now() <= deadlineSearch) {
                                                let btn = doc.querySelector('button[mattooltip="Search"]');
                                                if (btn && btn.offsetParent !== null) {
                                                    btn.click();
                                                    clickedSearch = true;
                                                    logs.push("Clicked button[mattooltip='Search']");
                                                    break;
                                                }

                                                let searchBtnIcon = doc.querySelector('i.bi-search');
                                                if (searchBtnIcon && searchBtnIcon.offsetParent !== null) {
                                                    let parentBtn = searchBtnIcon.closest('button');
                                                    if (parentBtn) {
                                                        parentBtn.click();
                                                        clickedSearch = true;
                                                        logs.push("Clicked button containing i.bi-search");
                                                        break;
                                                    }
                                                }
                                                await sleep(200);
                                            }

                                            if (!clickedSearch) {
                                                return JSON.stringify({ status: "ERROR", msg: "Search button not found.", logs });
                                            }
                                            return JSON.stringify({ status: "SUCCESS", logs });
                                        })(__TIMEOUT_MINUTES__);
