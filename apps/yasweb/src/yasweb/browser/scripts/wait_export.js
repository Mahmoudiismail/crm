
        (async function(timeoutMinutes) {
            function sleep(ms) { return new Promise(resolve => setTimeout(resolve, ms)); }
            const logs = [];
            const timeoutMs = timeoutMinutes * 60 * 1000;
            const pollMs = 1000;

            function getReportDocument() {
                return document;
            }

            function isVisible(element) {
                if (!element) return false;
                const view = element.ownerDocument && element.ownerDocument.defaultView;
                if (!view) return false;
                const style = view.getComputedStyle(element);
                return style.display !== 'none' &&
                    style.visibility !== 'hidden' &&
                    style.opacity !== '0' &&
                    element.getClientRects().length > 0;
            }

            function findVisibleLoader() {
                const selector = '#loader_svg, .loading-screen-wrapper, mat-progress-bar, .dx-loadpanel';
                const reportDoc = getReportDocument();
                const docs = [document];
                if (reportDoc && reportDoc !== document) docs.push(reportDoc);
                for (const doc of docs) {
                    for (const element of doc.querySelectorAll(selector)) {
                        if (isVisible(element)) return element;
                    }
                }
                return null;
            }

            function findExportButton() {
                const reportDoc = getReportDocument();
                if (!reportDoc) return null;

                const candidates = [];
                const labelled = reportDoc.querySelector('div[aria-label="Export"], button[aria-label="Export"]');
                if (labelled) candidates.push(labelled);

                for (const element of reportDoc.querySelectorAll('.dx-button-text')) {
                    if (element.textContent.trim().toLowerCase() === 'export') {
                        const button = element.closest('.dx-button, div[role="button"], button');
                        if (button && !candidates.includes(button)) candidates.push(button);
                    }
                }

                for (const element of reportDoc.querySelectorAll('button, div[role="button"], .dx-button')) {
                    if (element.textContent.trim().toLowerCase() === 'export' && !candidates.includes(element)) {
                        candidates.push(element);
                    }
                }
                return candidates.find(isVisible) || null;
            }

            function isExportEnabled(button) {
                if (!button || !isVisible(button)) return false;
                const disabledClasses = ['disabled', 'disable', 'dx-state-disabled', 'mat-button-disabled'];
                if (disabledClasses.some(className => button.classList.contains(className))) return false;
                if (button.disabled === true ||
                    button.getAttribute('disabled') !== null ||
                    button.getAttribute('aria-disabled') === 'true') return false;
                const view = button.ownerDocument.defaultView;
                if (view.getComputedStyle(button).pointerEvents === 'none') return false;
                return true;
            }

            // A missing loader is not proof that the report is ready. Wait for the
            // Export control itself to become enabled; the app toggles its disabled
            // state asynchronously after the report data has finished binding.
            logs.push("Waiting for the Export button to become enabled...");
            const deadline = Date.now() + timeoutMs;
            let lastState = "";
            while (Date.now() <= deadline) {
                const loader = findVisibleLoader();
                const button = findExportButton();
                const enabled = isExportEnabled(button);
                if (button) {
                    const state = JSON.stringify({
                        classes: button.className && button.className.toString(),
                        ariaDisabled: button.getAttribute('aria-disabled'),
                        disabled: button.getAttribute('disabled') !== null,
                        loaderVisible: !!loader
                    });
                    if (state !== lastState) {
                        logs.push("Export state: " + state);
                        lastState = state;
                    }
                } else if (lastState !== "missing") {
                    logs.push("Export button is not present yet.");
                    lastState = "missing";
                }

                if (button && enabled && !loader) {
                    // Re-check on the next poll so a transient enabled state during
                    // report refresh does not trigger an early export.
                    await sleep(500);
                    const latestButton = findExportButton();
                    if (latestButton && isExportEnabled(latestButton) && !findVisibleLoader()) {
                        latestButton.click();
                        logs.push("Clicked enabled Export button.");
                        return JSON.stringify({ status: "SUCCESS", logs });
                    }
                }
                await sleep(pollMs);
            }

            const button = findExportButton();
            return JSON.stringify({
                status: "ERROR",
                msg: button
                    ? "Export button did not become enabled before timeout."
                    : "Export button not found before timeout.",
                logs
            });
        })(__TIMEOUT_MINUTES__);
        