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
                if (input.selectedOptions) {
                    const values = Array.from(input.selectedOptions).map(o => o.value);
                    if (values.length > 0) {
                        argsMap[argName] = values.join(",");
                    } else {
                        argsMap[argName] = "";
                    }
                }
            } else if (input.value !== undefined && input.value !== null) {
                argsMap[argName] = input.value;
            }
        });

        if (hasMissingRequired) {
            return false;
        }

        hiddenArgsElement.value = JSON.stringify(argsMap);
        return true;
    }
};
