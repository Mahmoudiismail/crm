(async function(reportType) {
            function sleep(ms) { return new Promise(resolve => setTimeout(resolve, ms)); }
            const logs = [];
            const doc = document;

            const normalize = value => (value || "").replace(/\s+/g, " ").trim().toLowerCase();
            const target = normalize(reportType);

            // Wait for the iframe document to exist without repeatedly materializing innerHTML.
            for (let i = 0; i < 40; i++) {
                if (doc.body && doc.body.childElementCount > 0) break;
                await sleep(250);
            }

            for (let i = 0; i < 40; i++) {
                const radios = doc.querySelectorAll('mat-radio-button');
                let candidate = null;
                for (const radio of radios) {
                    if (normalize(radio.textContent).includes(target)) {
                        candidate = radio;
                        break;
                    }
                }
                if (candidate) {
                    const input = candidate.querySelector('input[type="radio"]');
                    if (input) {
                        input.click();
                        input.dispatchEvent(new Event('change', { bubbles: true }));
                    } else {
                        candidate.click();
                    }
                    logs.push("Selected report type using mat-radio-button");
                    return JSON.stringify({ status: "SUCCESS", logs });
                }

                for (const label of doc.querySelectorAll('label')) {
                    if (normalize(label.textContent).includes(target)) {
                        label.click();
                        logs.push("Selected report type using label fallback");
                        return JSON.stringify({ status: "SUCCESS", logs });
                    }
                }

                await sleep(250);
            }

            return JSON.stringify({ status: "ERROR", msg: "Report type not found: " + reportType, logs });
        })(__REPORT_TYPE_JSON__);
