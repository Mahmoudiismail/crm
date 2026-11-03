const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const source = fs.readFileSync(
    path.join(__dirname, "../src/runner/assets/js/validation.js"),
    "utf8",
);
const context = { window: {} };
vm.runInNewContext(source, context);

function serializeArgument({ value = "", argType = "date_var", dependenciesSatisfied, mappedDateArg } = {}) {
    const container = {
        dataset: {
            dependenciesSatisfied,
            mappedDateArg,
        },
    };
    const input = {
        value,
        required: false,
        checked: false,
        style: {},
        getAttribute(name) {
            return {
                "data-arg-name": "--start-date",
                "data-arg-type": argType,
            }[name] ?? null;
        },
        closest(selector) {
            return selector === ".arg-container" ? container : null;
        },
    };
    const hidden = { value: "" };
    const dynamicInputs = {
        querySelectorAll() {
            return [input];
        },
    };

    context.window.validation.serializeExternalApp(dynamicInputs, hidden);
    return JSON.parse(hidden.value);
}

test("manifest arguments are serialized only when available", async (t) => {
    await t.test("omits an argument with unsatisfied dependencies", () => {
        assert.deepEqual(
            serializeArgument({ value: "", dependenciesSatisfied: "false" }),
            {},
        );
    });

    await t.test("keeps an argument when its dependencies are satisfied", () => {
        assert.deepEqual(
            serializeArgument({ value: "2026-11-03", dependenciesSatisfied: "true" }),
            { "--start-date": "2026-11-03" },
        );
    });

    await t.test("omits a date argument mapped to periodization", () => {
        assert.deepEqual(
            serializeArgument({ value: "", dependenciesSatisfied: "true", mappedDateArg: "true" }),
            {},
        );
    });
});
