
        (async function() {
            const routePattern = /\/home\/mis-report\/module\//i;
            if (routePattern.test(location.href)) return JSON.stringify({ status: "SUCCESS", clicked: false });

            const menu = document.querySelector("#menuPinnedBtn:not(.d-none), #pinButton:not(.d-none)");
            const modules = document.querySelector(".menuModules");
            if (!modules || !modules.classList.contains("show-modules")) {
                if (!menu) return JSON.stringify({ status: "ERROR", msg: "MIS navigation recovery could not find the menu button." });
                menu.click();
                const menuDeadline = Date.now() + 5000;
                while (Date.now() < menuDeadline) {
                    const current = document.querySelector(".menuModules");
                    if (current && current.classList.contains("show-modules")) break;
                    await new Promise(resolve => setTimeout(resolve, 100));
                }
            }

            const deadline = Date.now() + 5000;
            while (Date.now() < deadline) {
                const module = document.querySelector(".misManagement");
                if (module) {
                    const style = getComputedStyle(module);
                    const visible = style.display !== "none" && style.visibility !== "hidden" &&
                        Array.from(module.getClientRects()).some(rect => rect.width > 0 && rect.height > 0);
                    if (visible) {
                        module.click();
                        return JSON.stringify({ status: "SUCCESS", clicked: true });
                    }
                }
                await new Promise(resolve => setTimeout(resolve, 100));
            }
            return JSON.stringify({ status: "ERROR", msg: "MIS navigation recovery could not find a visible MIS module." });
        })()
    