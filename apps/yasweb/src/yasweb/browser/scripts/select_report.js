(async function(reportType, reportName, timeoutMinutes) {
                                            function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }
                                            let logs = [];
                                            let doc = document;

                                            logs.push("Waiting for loader to disappear before typing...");
                                            for(let i=0; i<30; i++) {
                                                let loader = document.querySelector('#loader_svg, .loading-screen-wrapper, mat-progress-bar, .dx-loadpanel') || doc.querySelector('#loader_svg, .loading-screen-wrapper, mat-progress-bar, .dx-loadpanel');
                                                let isLoaderVisible = false;
                                                if (loader) {
                                                    let style = loader.ownerDocument.defaultView.getComputedStyle(loader);
                                                    if (style.display !== 'none' && style.opacity !== '0' && style.visibility !== 'hidden') {                                                        isLoaderVisible = true;
                                                    }
                                                }
                                                if (!isLoaderVisible) {
                                                    break;
                                                }
                                                await sleep(500);
                                            }

                                            let listLoaded = false;
                                            logs.push("Waiting for report list to load (tree-view)...");
                                            const deadlineList = Date.now() + timeoutMinutes * 60 * 1000;
                                            while (Date.now() <= deadlineList) {
                                                if (doc.querySelectorAll('.tree-view').length > 0) {
                                                    listLoaded = true; break;
                                                }
                                                await sleep(500);
                                            }
                                            if (!listLoaded) return JSON.stringify({ status: "ERROR", msg: "Report list timeout.", logs });
                                            logs.push("Report list loaded.");
                                            await sleep(200);

                                            logs.push("Typing in search input...");
                                            let searchInputList = doc.querySelector('input[formcontrolname="searchInput"], input[placeholder="Search"]');
                                            if (searchInputList) {
                                                searchInputList.focus();
                                                searchInputList.value = '';
                                                for (let i = 0; i < reportName.length; i++) {
                                                    searchInputList.value += reportName[i];
                                                    searchInputList.dispatchEvent(new Event('input', { bubbles: true }));
                                                    await sleep(5);
                                                }
                                                searchInputList.dispatchEvent(new Event('change', { bubbles: true }));

                                                // Trigger the 'Enter' key to execute the search
                                                searchInputList.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, key: 'Enter', code: 'Enter', keyCode: 13 }));
                                                searchInputList.dispatchEvent(new KeyboardEvent('keyup', { bubbles: true, cancelable: true, key: 'Enter', code: 'Enter', keyCode: 13 }));

                                                searchInputList.blur();
                                                searchInputList.dispatchEvent(new Event('blur', { bubbles: true }));
                                                logs.push("Typed search input using simulation and triggered Enter");
                                                await sleep(100); // Wait after typing
                                            } else {
                                                logs.push("Warning: searchInputList not found.");
                                            }

                                            let reportFound = false;
                                            logs.push("Waiting for report span in list: " + reportName);
                                            const deadlineReport = Date.now() + timeoutMinutes * 60 * 1000;
                                            while (Date.now() <= deadlineReport) {
                                                let exactMatchSpan = null;
                                                let partialMatchSpan = null;

                                                let spans = doc.querySelectorAll('.tree-view span');
                                                let searchName = reportName.trim().toLowerCase();

                                                for (let span of spans) {
                                                    let spanText = span.textContent.trim().toLowerCase();
                                                    if (spanText === searchName) {
                                                        exactMatchSpan = span;
                                                        break;
                                                    } else if (!partialMatchSpan && spanText.includes(searchName)) {
                                                        partialMatchSpan = span;
                                                    }
                                                }

                                                let bestMatchSpan = exactMatchSpan || partialMatchSpan;

                                                if (bestMatchSpan) {
                                                    if (exactMatchSpan) {
                                                        logs.push("Found exact match reportSpan");
                                                    } else {
                                                        logs.push("Found partial match reportSpan");
                                                    }

                                                    let listItemElement = bestMatchSpan.closest('.sub-list-items');
                                                    if (listItemElement) {
                                                        listItemElement.click();
                                                        logs.push("Clicked sub-list-items");
                                                    } else {
                                                        bestMatchSpan.click();
                                                        logs.push("Clicked reportSpan");
                                                    }
                                                    reportFound = true;
                                                    break;
                                                }
                                                await sleep(1000);
                                            }
                                            if (!reportFound) return JSON.stringify({ status: "ERROR", msg: "Report name not found: " + reportName, logs });
                                            return JSON.stringify({ status: "SUCCESS", logs });
                                        })(__REPORT_TYPE_JSON__, __REPORT_NAME_JSON__, __TIMEOUT_MINUTES__);
