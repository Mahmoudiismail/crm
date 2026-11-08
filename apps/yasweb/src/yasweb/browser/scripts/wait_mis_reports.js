return await (async function(timeoutSeconds) {
            function sleep(ms) { return new Promise(resolve => setTimeout(resolve, ms)); }
            const logs = [];
            const deadline = Date.now() + timeoutSeconds * 1000;
            const typeNames = ["Standard Report", "Report Manager"];

            function normalize(value) { return (value || "").replace(/\s+/g, " ").trim().toLowerCase(); }
            function isVisible(element) {
                if (!element) return false;
                const view = element.ownerDocument && element.ownerDocument.defaultView;
                if (!view) return false;
                for (let current = element; current && current.nodeType === 1; current = current.parentElement) {
                    const style = view.getComputedStyle(current);
                    if (style.display === "none" || style.visibility === "hidden" || style.visibility === "collapse" || Number(style.opacity) === 0) return false;
                }
                return Array.from(element.getClientRects()).some(rect => rect.width > 0 && rect.height > 0);
            }
            function findTypeControl(typeName) {
                const target = normalize(typeName);
                return Array.from(document.querySelectorAll('mat-radio-button, [role="radio"], input[type="radio"], label, button'))
                    .find(element => isVisible(element) && normalize(element.textContent || element.getAttribute("aria-label") || element.value).includes(target)) || null;
            }
            function inspect() {
                const reportTypes = typeNames.map(name => ({ name, visible: !!findTypeControl(name) }));
                const searchInput = document.querySelector('input[formcontrolname="searchInput"], input[placeholder="Search"], input[aria-label*="Search" i]');
                const loaderSelector = "#loader_svg, .loading-screen-wrapper, mat-progress-bar, .dx-loadpanel";
                const visibleLoaders = Array.from(document.querySelectorAll(loaderSelector)).filter(isVisible).map(element => ({
                    selector: loaderSelector,
                    tag: element.tagName,
                    className: typeof element.className === "string" ? element.className : ""
                }));
                return { frameUrl: location.href, documentReadyState: document.readyState, reportTypes,
                    searchVisible: isVisible(searchInput), visibleLoaders,
                    reportUiReady: reportTypes.every(item => item.visible) && isVisible(searchInput) && visibleLoaders.length === 0 };
            }
            logs.push("Waiting inside the report-frame execution context for visible report-type controls and Search...");
            let lastState = "";
            while (Date.now() <= deadline) {
                const state = inspect();
                const serialized = JSON.stringify(state);
                if (serialized !== lastState) { logs.push("Report-frame readiness state: " + serialized); lastState = serialized; }
                if (state.reportUiReady) return JSON.stringify({ status: "SUCCESS", logs });
                await sleep(250);
            }
            return JSON.stringify({ status: "ERROR", msg: "Report frame did not expose visible Standard Report, Report Manager, and Search controls before timeout",
                logs: logs.concat(["Report-frame readiness timeout diagnostics: " + JSON.stringify(inspect())]) });
        })(__TIMEOUT_SECONDS__);
