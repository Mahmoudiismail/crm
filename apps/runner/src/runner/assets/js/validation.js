// Validation Utilities

window.validation = {
    serializeExternalApp: function(dynamicInputsElement, hiddenArgsElement) {
        if (!dynamicInputsElement || !hiddenArgsElement) return true;
        const argsMap = {};
        const inputs = dynamicInputsElement.querySelectorAll(
            "input[data-arg-name], select[data-arg-name]",
        );
        let hasMissingRequired = false;

        inputs.forEach((input) => {
            const argName = input.getAttribute("data-arg-name");
            const argType = input.getAttribute("data-arg-type");

            if (input.required && !input.value.trim() && argType !== "boolean") {
                hasMissingRequired = true;
            }

            if (argType === "boolean") {
                if (input.checked) {
                    argsMap[argName] = "true";
                }
            } else if (argType === "multi_list") {
                if (input.checked) {
                    if (!argsMap[argName]) {
                        argsMap[argName] = [];
                    }
                    argsMap[argName].push(input.value);
                }
            } else if (argType === "date_var") {
                if (input.style.display !== 'none') {
                    argsMap[argName] = input.value;
                }
            } else if (input.value !== undefined && input.value !== null) {
                argsMap[argName] = input.value;
            }
        });

        // Map arrays to comma-separated strings for multi_list
        Object.keys(argsMap).forEach(key => {
            if (Array.isArray(argsMap[key])) {
                argsMap[key] = argsMap[key].join(",");
            }
        });


        if (hasMissingRequired) {
            return false;
        }

        hiddenArgsElement.value = JSON.stringify(argsMap);
        return true;
    }
};
