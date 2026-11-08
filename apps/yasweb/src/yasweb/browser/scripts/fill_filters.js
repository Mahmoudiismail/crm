(async function(filters, timeoutMinutes) {
                                            function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }
                                            let logs = [];
                                            let doc = document;

                                            logs.push("Waiting for loader to disappear...");
                                            let loaderGone = false;
                                            const deadlineFillLoader = Date.now() + timeoutMinutes * 60 * 1000;
                                            while (Date.now() <= deadlineFillLoader) {
                                                let loader = document.querySelector('#loader_svg, .loading-screen-wrapper, mat-progress-bar, .dx-loadpanel') || doc.querySelector('#loader_svg, .loading-screen-wrapper, mat-progress-bar, .dx-loadpanel');
                                                let isLoaderVisible = false;
                                                if (loader) {
                                                    let style = loader.ownerDocument.defaultView.getComputedStyle(loader);
                                                    if (style.display !== 'none' && style.opacity !== '0' && style.visibility !== 'hidden') {
                                                        isLoaderVisible = true;
                                                    }
                                                }
                                                if (!isLoaderVisible && doc.querySelectorAll('mat-label').length > 0) {
                                                    loaderGone = true;
                                                    break;
                                                }
                                                await sleep(500);
                                            }
                                            if (!loaderGone) logs.push("Warning: Loader timeout or labels never appeared.");

                                            async function simulateTyping(inputElem, text) {
                                                inputElem.focus();
                                                inputElem.value = '';
                                                for (let i = 0; i < text.length; i++) {
                                                    inputElem.value += text[i];
                                                    inputElem.dispatchEvent(new Event('input', { bubbles: true }));
                                                    await sleep(10);
                                                }
                                                inputElem.dispatchEvent(new Event('change', { bubbles: true }));
                                                inputElem.blur();
                                                inputElem.dispatchEvent(new Event('blur', { bubbles: true }));
                                            }

                                            const labels = doc.querySelectorAll('mat-label');
                                            const discoveredFilters = [];
                                            const discoveredFilterOptions = {};
                                            const failedFilters = [];
                                            const normalize = value => (value || '').replace(/\s+/g, ' ').trim().toLowerCase();

                                            function findFilterControl(label) {
                                                const field = label.closest('mat-form-field');
                                                if (field) {
                                                    const control = field.querySelector('mat-select, select, input, [role="combobox"]');
                                                    if (control) return control;
                                                }
                                                const labelled = label.closest('label[for]');
                                                if (labelled) return doc.getElementById(labelled.getAttribute('for'));
                                                return null;
                                            }

                                            async function readFilterOptions(control) {
                                                if (!control) return [];
                                                if (control.tagName === 'SELECT') {
                                                    return Array.from(control.options)
                                                        .filter(option => !option.disabled)
                                                        .map(option => (option.textContent || '').trim())
                                                        .filter(Boolean);
                                                }
                                                if (control.tagName !== 'MAT-SELECT' && control.getAttribute('role') !== 'combobox') return [];

                                                control.click();
                                                const deadline = Date.now() + 2500;
                                                let optionElements = [];
                                                while (Date.now() < deadline) {
                                                    const overlay = doc.querySelector('.cdk-overlay-container') || doc;
                                                    optionElements = Array.from(overlay.querySelectorAll('mat-option, [role="option"]'));
                                                    if (optionElements.length > 0) break;
                                                    await sleep(100);
                                                }
                                                const values = optionElements
                                                    .filter(option => option.getAttribute('aria-disabled') !== 'true' && !option.classList.contains('mat-option-disabled'))
                                                    .map(option => (option.innerText || option.textContent || '').replace(/\s+/g, ' ').trim())
                                                    .filter(Boolean);
                                                const backdrop = doc.querySelector('.cdk-overlay-backdrop');
                                                if (backdrop) backdrop.click();
                                                else control.click();
                                                await sleep(100);
                                                return [...new Set(values)];
                                            }

                                            for (const label of labels) {
                                                const filterName = (label.innerText || label.textContent || '').trim();
                                                if (!filterName) continue;
                                                if (!discoveredFilters.includes(filterName)) discoveredFilters.push(filterName);
                                                const control = findFilterControl(label);
                                                if (!control) {
                                                    logs.push("No control found for filter label: " + filterName);
                                                    continue;
                                                }
                                                const options = await readFilterOptions(control);
                                                if (options.length > 0) {
                                                    discoveredFilterOptions[filterName] = options;
                                                    logs.push("Discovered " + options.length + " options for filter: " + filterName);
                                                }
                                            }
                                            logs.push("Discovered filters count: " + discoveredFilters.length);
                                            logs.push("Discovered dropdown lists: " + Object.keys(discoveredFilterOptions).length);

                                            for (const [key, value] of Object.entries(filters)) {
                                                if (!value || value.trim() === '') {
                                                    logs.push("Skipping empty filter: " + key);
                                                    continue;
                                                }
                                                logs.push("Applying filter: " + key + " = " + value);
                                                let normalizedKey = key.toLowerCase().replace(/_/g, ' ');
                                                let filterFilled = false;

                                                for (let lbl of labels) {
                                                    let labelText = lbl.innerText.trim().toLowerCase().replace(/_/g, ' ');
                                                    if (labelText === normalizedKey) {
                                                        const input = findFilterControl(lbl);
                                                        if (input) {
                                                            if (input.tagName === 'INPUT') {
                                                                let v = value;
                                                                if (key.toLowerCase().includes('date') && v.includes('-')) {
                                                                    const parts = v.split(' ')[0].split('-');
                                                                    if (parts.length === 3) {
                                                                        v = parts[0].padStart(2, '0') + "-" + parts[1].padStart(2, '0') + "-" + parts[2] + (v.includes(' ') ? ' ' + v.split(' ').slice(1).join(' ') : '');
                                                                    }
                                                                }
                                                                await simulateTyping(input, v);
                                                                logs.push("Typed into INPUT for " + key);
                                                                filterFilled = true;
                                                                break;
                                                            } else if (input.tagName === 'MAT-SELECT' || input.getAttribute('role') === 'combobox') {
                                                                input.click();
                                                                const optionDeadline = Date.now() + 2500;
                                                                let options = [];
                                                                while (Date.now() < optionDeadline) {
                                                                    const overlay = doc.querySelector('.cdk-overlay-container') || doc;
                                                                    options = Array.from(overlay.querySelectorAll('mat-option, [role="option"]'));
                                                                    if (options.length > 0) break;
                                                                    await sleep(100);
                                                                }
                                                                const exactOption = options.find(option => normalize(option.innerText || option.textContent) === normalize(value));
                                                                const partialMatches = options.filter(option => normalize(option.innerText || option.textContent).includes(normalize(value)));
                                                                const selectedOption = exactOption || (partialMatches.length === 1 ? partialMatches[0] : null);
                                                                if (selectedOption) {
                                                                    selectedOption.click();
                                                                    logs.push("Selected dropdown option for " + key);
                                                                    filterFilled = true;
                                                                } else {
                                                                    const backdrop = doc.querySelector('.cdk-overlay-backdrop');
                                                                    if (backdrop) backdrop.click();
                                                                    logs.push("Dropdown option not found for " + key + ": " + value);
                                                                }
                                                                await sleep(300);
                                                                break;
                                                            } else if (input.tagName === 'SELECT') {
                                                                const option = Array.from(input.options).find(item =>
                                                                    normalize(item.textContent) === normalize(value) || item.value === value
                                                                );
                                                                if (option) {
                                                                    input.value = option.value;
                                                                    input.dispatchEvent(new Event('input', { bubbles: true }));
                                                                    input.dispatchEvent(new Event('change', { bubbles: true }));
                                                                    logs.push("Selected native SELECT option for " + key);
                                                                    filterFilled = true;
                                                                } else {
                                                                    logs.push("Native SELECT option not found for " + key + ": " + value);
                                                                }
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                                if (!filterFilled) {
                                                    logs.push("Failed to fill filter: " + key);
                                                    failedFilters.push(key);
                                                }
                                            }
                                            if (failedFilters.length > 0) {
                                                return JSON.stringify({ status: "ERROR", msg: "One or more requested filters could not be applied: " + failedFilters.join(", "), failed_filters: failedFilters, discovered_filters: discoveredFilters, discovered_filter_options: discoveredFilterOptions, logs });
                                            }
                                            return JSON.stringify({ status: "SUCCESS", discovered_filters: discoveredFilters, discovered_filter_options: discoveredFilterOptions, logs });
                                        })(__FILTERS_JSON__, __TIMEOUT_MINUTES__, __TRACE_ENABLED__);
