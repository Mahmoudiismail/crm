(function () {
  const scheduleRows = document.getElementById("schedule-rows");
  const commandRows = document.getElementById("command-rows");
  const schedulesHidden = document.getElementById("schedules-hidden");
  const commandsHidden = document.getElementById("commands-hidden");
  const addScheduleBtn = document.getElementById("add-schedule-row");
  const addCommandBtn = document.getElementById("add-command-row");
  let scheduleIndex = scheduleRows ? scheduleRows.children.length : 0;
  let commandIndex = commandRows ? commandRows.children.length : 0;

  function updateVisibility(row) {
    const kind = row.querySelector(".schedule-kind").value;
    row
      .querySelector(".schedule-interval")
      .classList.toggle("hidden", kind !== "interval");
    row
      .querySelector(".schedule-once")
      .classList.toggle("hidden", kind !== "once");
    row
      .querySelector(".schedule-daily")
      .classList.toggle("hidden", kind !== "daily");
    row
      .querySelector(".schedule-weekly")
      .classList.toggle("hidden", kind !== "weekly");
    row
      .querySelector(".schedule-monthly")
      .classList.toggle("hidden", kind !== "monthly");
    const whContainer = row.querySelector(".schedule-wh");
    if (whContainer) {
      whContainer.classList.toggle(
        "hidden",
        kind !== "interval" && kind !== "daily",
      );
    }
  }

  function createWhRow() {
    const row = document.createElement("div");
    row.className = "flex gap-2 items-center mt-2";
    row.setAttribute("data-wh-row", "");
    const daysOfWeek = [
      "Monday",
      "Tuesday",
      "Wednesday",
      "Thursday",
      "Friday",
      "Saturday",
      "Sunday",
    ];
    row.innerHTML = `
            <select class='rounded border border-gray-300 px-2 py-1 text-sm wh-day'>
                ${daysOfWeek.map((day) => `<option value='${day}'>${day}</option>`).join("")}
            </select>
            <input class='rounded border border-gray-300 px-2 py-1 text-sm w-24 wh-start' type='time' value='09:00'>
            <span class='text-xs text-gray-500'>to</span>
            <input class='rounded border border-gray-300 px-2 py-1 text-sm w-24 wh-end' type='time' value='17:00'>
            <button type='button' class='remove-wh rounded bg-red-100 px-2 py-1 text-xs font-semibold text-red-700'>&times;</button>
        `;
    return row;
  }

  function attachWhEvents(row) {
    const removeBtn = row.querySelector(".remove-wh");
    if (removeBtn) {
      removeBtn.addEventListener("click", function () {
        row.remove();
      });
    }
  }

  function attachScheduleEvents(row) {
    row.querySelector(".schedule-kind").addEventListener("change", function () {
      updateVisibility(row);
    });
    row
      .querySelector(".remove-schedule")
      .addEventListener("click", function () {
        row.remove();
      });
    const addWhBtn = row.querySelector(".add-wh-row");
    if (addWhBtn) {
      addWhBtn.addEventListener("click", function () {
        const whRows = row.querySelector(".wh-rows");
        const whRow = createWhRow();
        whRows.appendChild(whRow);
        attachWhEvents(whRow);
      });
    }
    Array.from(row.querySelectorAll("[data-wh-row]")).forEach(attachWhEvents);
  }

  function attachCommandEvents(row) {
    row.querySelector(".remove-command").addEventListener("click", function () {
      row.remove();
    });
  }

  function createScheduleRow(
    kind,
    interval,
    once,
    daily,
    weekly,
    monthly,
    startTime,
  ) {
    const row = document.createElement("div");
    row.setAttribute("data-schedule-row", "");
    row.className =
      "grid md:grid-cols-6 gap-2 p-3 border border-gray-200 rounded items-end";
    const daysOfWeek = [
      "Monday",
      "Tuesday",
      "Wednesday",
      "Thursday",
      "Friday",
      "Saturday",
      "Sunday",
    ];
    row.innerHTML = `
            <label class='block'>
                <span class='text-xs font-semibold text-gray-700'>Type</span>
                <select class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm schedule-kind'>
                    <option value='interval' ${kind === "interval" ? "selected" : ""}>Interval</option>
                    <option value='once' ${kind === "once" ? "selected" : ""}>Once</option>
                    <option value='daily' ${kind === "daily" ? "selected" : ""}>Daily</option>
                    <option value='weekly' ${kind === "weekly" ? "selected" : ""}>Weekly</option>
                    <option value='monthly' ${kind === "monthly" ? "selected" : ""}>Monthly</option>
                </select>
            </label>
            <label class='block schedule-interval ${kind === "interval" ? "" : "hidden"}'>
                <span class='text-xs font-semibold text-gray-700'>Interval</span>
                <select class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm'>
                    ${["15m", "30m", "1h", "2h", "4h", "8h", "12h", "24h", "2d", "7d"].map((opt) => `<option value='${opt}' ${opt === interval ? "selected" : ""}>${opt}</option>`).join("")}
                </select>
            </label>
            <label class='block schedule-interval schedule-start-time ${kind === "interval" ? "" : "hidden"}'>
                <span class='text-xs font-semibold text-gray-700'>Start Time (HH:MM)</span>
                <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='time' value='${startTime || ""}'>
            </label>
            <label class='block schedule-once ${kind === "once" ? "" : "hidden"}'>
                <span class='text-xs font-semibold text-gray-700'>Date & Time</span>
                <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='datetime-local' value='${once}'>
            </label>
            <label class='block schedule-daily ${kind === "daily" ? "" : "hidden"}'>
                <span class='text-xs font-semibold text-gray-700'>Times (HH:MM)</span>
                <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='text' value='${daily}' placeholder='09:00, 13:00'>
            </label>
            <label class='block schedule-weekly ${kind === "weekly" ? "" : "hidden"}'>
                <span class='text-xs font-semibold text-gray-700'>Day</span>
                <select class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' data-weekly-day>
                    ${daysOfWeek.map((day) => `<option value='${day}' ${day === weekly ? "selected" : ""}>${day}</option>`).join("")}
                </select>
            </label>
            <label class='block schedule-monthly ${kind === "monthly" ? "" : "hidden"}'>
                <span class='text-xs font-semibold text-gray-700'>Day (1-31)</span>
                <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='number' value='${monthly}' min='1' max='31'>
            </label>
            <button type='button' class='remove-schedule rounded border border-red-200 bg-red-50 px-3 py-2 text-sm font-semibold text-red-700'>Remove</button>
          </div>
          <div class='mt-3 schedule-wh ${kind === "interval" || kind === "daily" ? "" : "hidden"}'>
              <div class='flex items-center justify-between'>
                  <span class='text-xs font-semibold text-gray-700'>Working Hours (Optional)</span>
                  <button type='button' class='add-wh-row rounded border border-gray-300 bg-white px-2 py-1 text-xs font-semibold text-gray-700 hover:bg-gray-50'>+ Add Day</button>
              </div>
              <div class='wh-rows'></div>
          </div>
        `;
    return row;
  }

  function createCommandRow(command) {
    const row = document.createElement("div");
    row.setAttribute("data-command-row", "");
    row.className =
      "grid md:grid-cols-[1fr_100px_auto] gap-2 items-center p-2 bg-gray-50 border border-gray-200 rounded";
    row.innerHTML = `
            <div class='grid md:grid-cols-2 gap-2'>
                <label class='block'>
                    <span class='text-xs font-semibold text-gray-700'>Command</span>
                    <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm command-text' type='text' value='${command}' placeholder='echo hello'>
                </label>
                <label class='block'>
                    <span class='text-xs font-semibold text-gray-700'>Mode</span>
                    <select class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm command-mode'>
                        <option value='run'>Run</option>
                        <option value='continue'>Continue</option>
                    </select>
                </label>
            </div>
            <button type='button' class='remove-command rounded bg-red-600 text-white px-3 py-2 text-sm font-semibold hover:bg-red-700'>Remove</button>
        `;
    return row;
  }

  if (scheduleRows) {
    Array.from(scheduleRows.querySelectorAll("[data-schedule-row]")).forEach(
      attachScheduleEvents,
    );
  }
  if (commandRows) {
    Array.from(commandRows.querySelectorAll("[data-command-row]")).forEach(
      attachCommandEvents,
    );
  }
  if (addScheduleBtn) {
    addScheduleBtn.addEventListener("click", function () {
      const scheduleRows = document.getElementById("schedule-rows");
      if (!scheduleRows) return;
      const row = createScheduleRow("interval", "1h", "", "", "", "", "");
      scheduleRows.appendChild(row);
      attachScheduleEvents(row);
      scheduleIndex += 1;
    });
  }
  if (addCommandBtn) {
    addCommandBtn.addEventListener("click", function () {
      const commandRows = document.getElementById("command-rows");
      if (!commandRows) return;
      const row = createCommandRow("");
      commandRows.appendChild(row);
      attachCommandEvents(row);
      commandIndex += 1;
    });
  }

  function encodeIsoDatetime(value) {
    if (!value) return "";
    const date = new Date(value);
    return date.toISOString();
  }

  function buildSchedules() {
    if (!scheduleRows) return "";
    return Array.from(scheduleRows.querySelectorAll("[data-schedule-row]"))
      .map(function (row) {
        const kind = row.querySelector(".schedule-kind").value;
        if (kind === "interval") {
          const interval = row.querySelector(".schedule-interval select").value;
          const startTime = row.querySelector(
            ".schedule-start-time input",
          ).value;
          const whRows = Array.from(row.querySelectorAll("[data-wh-row]"))
            .map((whRow) => {
              const day = whRow.querySelector(".wh-day").value;
              const start = whRow.querySelector(".wh-start").value;
              const end = whRow.querySelector(".wh-end").value;
              if (day && start && end) {
                return `${day}=${start}-${end}`;
              }
              return null;
            })
            .filter(Boolean)
            .join(",");

          let result = "interval: every " + interval;
          if (startTime) {
            result += "; st: " + startTime;
          }
          if (whRows) {
            result += "; wh: " + whRows;
          }
          return result;
        }
        if (kind === "once") {
          const value = row.querySelector(".schedule-once input").value;
          return value ? "once: " + encodeIsoDatetime(value) : "once:";
        }
        if (kind === "daily") {
          const value = row.querySelector(".schedule-daily input").value.trim();
          const whRows = Array.from(row.querySelectorAll("[data-wh-row]"))
            .map((whRow) => {
              const day = whRow.querySelector(".wh-day").value;
              const start = whRow.querySelector(".wh-start").value;
              const end = whRow.querySelector(".wh-end").value;
              if (day && start && end) {
                return `${day}=${start}-${end}`;
              }
              return null;
            })
            .filter(Boolean)
            .join(",");

          if (whRows) {
            return value ? "daily: " + value + "; wh: " + whRows : "";
          } else {
            return value ? "daily: " + value : "";
          }
        }
        if (kind === "weekly") {
          const value = row.querySelector("[data-weekly-day]").value;
          return value ? "weekly: " + value : "";
        }
        if (kind === "monthly") {
          const value = row.querySelector(".schedule-monthly input").value;
          return value ? "monthly: day " + value : "";
        }
        return "";
      })
      .filter(Boolean)
      .join("\n");
  }

  function buildCommands() {
    if (!commandRows) return "";
    return Array.from(commandRows.querySelectorAll("[data-command-row]"))
      .map(function (row) {
        const command = row.querySelector(".command-text").value.trim();
        const mode = row.querySelector(".command-mode").value;
        if (!command) return "";
        return mode === "continue" ? "continue: " + command : command;
      })
      .filter(Boolean)
      .join("\n");
  }

  const taskTypeSelect = document.getElementById("task-type-select");
  const shellCommandContainer = document.getElementById(
    "shell-command-container",
  );
  const externalAppContainer = document.getElementById(
    "external-app-container",
  );
  const externalAppSelectContainer = document.getElementById(
    "external-app-select-container",
  );
  const externalAppDynamicInputs = document.getElementById(
    "external-app-dynamic-inputs",
  );
  const externalAppArgsHidden = document.getElementById("external_app_args");
  const externalAppIdHidden = document.getElementById("external_app_id");

  let registeredAppsCache = null;

  async function fetchYaswebConfig() {
    try {
      const res = await fetch("/api/yasweb-config");
      if (res.ok) {
        yaswebConfigData = await res.json();
        populateYaswebReports();
      }
    } catch (e) {
      console.error("Failed to fetch yasweb config", e);
    }
  }

  function createYaswebReportRow(reportData) {
    const row = document.createElement("div");
    row.setAttribute("data-yasweb-report-row", "");
    row.className = "p-3 border border-blue-300 rounded bg-white relative";

    const reportName = reportData ? reportData.report_name : "";
    const reportType = reportData ? reportData.report_type : "";
    const filters = reportData ? reportData.filters : {};

    let optionsHtml = '<option value="">-- Select Report --</option>';
    if (yaswebConfigData && yaswebConfigData.reports) {
      const reports = Object.keys(yaswebConfigData.reports);
      if (reportName && !reports.includes(reportName)) {
        reports.push(reportName);
      }
      reports.forEach((name) => {
        optionsHtml += `<option value="${name}" ${name === reportName ? "selected" : ""}>${name}</option>`;
      });
    }

    row.innerHTML = `
            <button type="button" class="remove-yasweb-report absolute top-2 right-2 text-red-600 hover:text-red-800 font-bold">&times;</button>
            <div class="grid md:grid-cols-2 gap-4 mb-3">
                <label class="block">
                    <span class="text-sm font-semibold text-gray-800">Select Configured Report</span>
                    <select class="yasweb-report-select mt-1 block w-full rounded border border-gray-300 px-3 py-2 text-sm">
                        ${optionsHtml}
                    </select>
                </label>
                <div class="flex items-end mb-1">
                    <button type="button" class="refresh-filters rounded border border-gray-300 bg-blue-100 text-blue-800 px-3 py-1.5 text-sm font-semibold hover:bg-blue-200">Refresh Filters</button>
                </div>
            </div>
            <div class="grid md:grid-cols-2 gap-4 mb-3">
                <label class="block"><span class="text-sm font-semibold text-gray-800">Report Type</span><input class="yasweb-type-input mt-1 block w-full rounded border border-gray-300 px-3 py-2 text-sm" type="text" value="${reportType}"></label>
                <label class="block"><span class="text-sm font-semibold text-gray-800">Report Name</span><input class="yasweb-name-input mt-1 block w-full rounded border border-gray-300 px-3 py-2 text-sm" type="text" value="${reportName}"></label>
            </div>
            <div class="yasweb-filters-container space-y-2">
                <h4 class="text-sm font-semibold text-gray-700 border-b pb-1">Filters</h4>
                <div class="yasweb-filters-list grid md:grid-cols-2 gap-2"></div>
            </div>
        `;

    const filtersList = row.querySelector(".yasweb-filters-list");

    function renderFilters(currentFilters) {
      filtersList.innerHTML = "";
      for (const [key, value] of Object.entries(currentFilters)) {
        const filterDiv = document.createElement("div");
        filterDiv.className = "flex flex-col";
        filterDiv.innerHTML = `
                    <label class="text-xs font-semibold text-gray-600">${key}</label>
                    <input type="text" data-filter-key="${key}" value="${value}" class="yasweb-filter-input rounded border border-gray-300 px-2 py-1 text-sm">
                `;
        filtersList.appendChild(filterDiv);
      }
    }

    renderFilters(filters);

    row
      .querySelector(".yasweb-report-select")
      .addEventListener("change", function (e) {
        const selectedName = e.target.value;
        const nameInput = row.querySelector(".yasweb-name-input");
        const typeInput = row.querySelector(".yasweb-type-input");
        nameInput.value = selectedName;

        if (
          selectedName &&
          yaswebConfigData &&
          yaswebConfigData.reports[selectedName]
        ) {
          const conf = yaswebConfigData.reports[selectedName];
          typeInput.value = conf.report_type || "";
          if (!reportData || reportData.report_name !== selectedName) {
            renderFilters(conf.filters || {});
          }
        }
      });

    row
      .querySelector(".refresh-filters")
      .addEventListener("click", function () {
        const selectedName = row.querySelector(".yasweb-name-input").value;
        if (
          selectedName &&
          yaswebConfigData &&
          yaswebConfigData.reports[selectedName]
        ) {
          const conf = yaswebConfigData.reports[selectedName];

          const currentFilters = {};
          Array.from(row.querySelectorAll(".yasweb-filter-input")).forEach(
            (inp) => {
              currentFilters[inp.getAttribute("data-filter-key")] = inp.value;
            },
          );

          const newFilters = { ...conf.filters };
          for (const k in currentFilters) {
            if (newFilters.hasOwnProperty(k)) {
              newFilters[k] = currentFilters[k];
            }
          }

          renderFilters(newFilters);
        }
      });

    row
      .querySelector(".remove-yasweb-report")
      .addEventListener("click", function () {
        row.remove();
      });

    return row;
  }

  function populateYaswebReports() {
    if (!yaswebReportsList) return;

    Array.from(
      yaswebReportsList.querySelectorAll("[data-yasweb-report-row]"),
    ).forEach((row) => {
      const select = row.querySelector(".yasweb-report-select");
      const nameInput = row.querySelector(".yasweb-name-input");
      const currentVal = nameInput.value;

      let optionsHtml = '<option value="">-- Select Report --</option>';
      if (yaswebConfigData && yaswebConfigData.reports) {
        const reports = Object.keys(yaswebConfigData.reports);
        if (currentVal && !reports.includes(currentVal)) {
          reports.push(currentVal);
        }
        reports.forEach((name) => {
          optionsHtml += `<option value="${name}" ${name === currentVal ? "selected" : ""}>${name}</option>`;
        });
      }
      select.innerHTML = optionsHtml;
      if (currentVal) {
        select.value = currentVal;
      }
    });
  }

  function updateTaskTypeVisibility() {
    const type = taskTypeSelect.value;
    if (shellCommandContainer) {
      shellCommandContainer.classList.toggle(
        "hidden",
        type !== "shell_command",
      );
    }
    if (externalAppContainer) {
      externalAppContainer.classList.toggle("hidden", type !== "external_app");
      if (type === "external_app" && !registeredAppsCache) {
        loadRegisteredApps();
      }
    }
  }

  async function loadRegisteredApps() {
    if (!externalAppSelectContainer) return;
    try {
      externalAppSelectContainer.innerHTML =
        '<span class="text-sm text-gray-500">Loading apps...</span>';
      const response = await fetch("/api/apps/list");
      const apps = await response.json();
      registeredAppsCache = apps;

      if (apps.length === 0) {
        externalAppSelectContainer.innerHTML =
          '<span class="text-sm text-gray-500">No external applications registered. <a href="/apps" class="text-purple-600 underline">Manage Apps</a></span>';
        return;
      }

      let selectHtml =
        '<label class="block"><span class="text-sm font-semibold text-gray-800">Select Application</span><select id="external-app-select" class="mt-1 w-full rounded border border-purple-300 px-3 py-2 text-sm"><option value="" disabled selected>-- Choose an app --</option>';
      apps.forEach((app) => {
        const selected = externalAppIdHidden.value === app.id ? "selected" : "";
        selectHtml += `<option value="${app.id}" ${selected}>${app.name} (${app.id})</option>`;
      });
      selectHtml += "</select></label>";
      externalAppSelectContainer.innerHTML = selectHtml;

      const selectEl = document.getElementById("external-app-select");
      selectEl.addEventListener("change", (e) =>
        loadAppManifest(e.target.value),
      );

      // Auto-load manifest if we are editing an existing external app task
      if (
        externalAppIdHidden.value &&
        apps.find((a) => a.id === externalAppIdHidden.value)
      ) {
        loadAppManifest(externalAppIdHidden.value, true);
      }
    } catch (e) {
      console.error("Failed to load registered apps", e);
      externalAppSelectContainer.innerHTML =
        '<span class="text-sm text-red-500">Failed to load apps</span>';
    }
  }

  async function loadAppManifest(appId, isInitialLoad = false) {
    if (!externalAppDynamicInputs) return;
    externalAppIdHidden.value = appId;
    externalAppDynamicInputs.innerHTML =
      '<span class="text-sm text-gray-500">Loading manifest...</span>';

    try {
      const response = await fetch(
        "/api/apps/manifest?app_id=" + encodeURIComponent(appId),
      );
      const manifest = await response.json();

      if (manifest.error) {
        externalAppDynamicInputs.innerHTML = `<span class="text-sm text-red-500">Error: ${manifest.error}</span>`;
        return;
      }

      let existingArgs = {};
      if (isInitialLoad && externalAppArgsHidden.value) {
        try {
          existingArgs = JSON.parse(externalAppArgsHidden.value);
        } catch (e) {}
      }

      let html = "";

      if (manifest.description) {
        html += `<p class="text-xs text-gray-600 mb-3">${manifest.description}</p>`;
      }

      if (manifest.arguments && manifest.arguments.length > 0) {
        html += '<div class="space-y-3">';
        manifest.arguments.forEach((arg, i) => {
          const argId = `ext-arg-${i}`;
          const requiredAttr = arg.required ? "required" : "";
          const labelSpan = `<span class="text-sm font-semibold text-gray-800">${arg.name} ${arg.required ? '<span class="text-red-500">*</span>' : ""}</span>`;

          let currentValue = existingArgs[arg.name];
          if (
            currentValue === undefined &&
            arg.default_value !== null &&
            arg.default_value !== undefined
          ) {
            currentValue = arg.default_value;
          }

          let dependsAttr = "";
          let hiddenClass = "";
          if (arg.depends_on) {
            dependsAttr = `data-depends-on='${JSON.stringify(arg.depends_on).replace(/'/g, "&#39;")}'`;
            hiddenClass = "hidden";
          }

          let autofillAttr = "";
          if (arg.autofill) {
            autofillAttr = `data-autofill='${JSON.stringify(arg.autofill).replace(/'/g, "&#39;")}'`;
          }

          let wrapperStart = `<div class="arg-wrapper ${hiddenClass}" ${dependsAttr}>`;
          let wrapperEnd = `</div>`;

          if (arg.arg_type === "boolean") {
            const checked =
              currentValue === "true" ||
              currentValue === "on" ||
              currentValue === true
                ? "checked"
                : "";
            html += `${wrapperStart}<label class="flex items-center gap-2"><input type="checkbox" id="${argId}" data-arg-name="${arg.name}" data-arg-type="boolean" ${autofillAttr} ${checked}> <span class="text-sm font-semibold text-gray-800">${arg.name}</span></label>${wrapperEnd}`;
          } else if (arg.arg_type === "list") {
            let optionsHtml = "";
            if (arg.options) {
              arg.options.forEach((opt) => {
                const sel = currentValue === opt ? "selected" : "";
                optionsHtml += `<option value="${opt}" ${sel}>${opt}</option>`;
              });
            }
            html += `${wrapperStart}<label class="block">${labelSpan}<select id="${argId}" data-arg-name="${arg.name}" data-arg-type="list" class="mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm" ${autofillAttr} ${requiredAttr}>${optionsHtml}</select></label>${wrapperEnd}`;
          } else if (arg.arg_type === "multi_list") {
            let optionsHtml = "";
            const currentValues = currentValue ? currentValue.split(',').map(s => s.trim()) : [];
            if (arg.options) {
              arg.options.forEach((opt) => {
                const sel = currentValues.includes(opt) ? "selected" : "";
                optionsHtml += `<option value="${opt}" ${sel}>${opt}</option>`;
              });
            }
            html += `${wrapperStart}<label class="block">${labelSpan}<select id="${argId}" data-arg-name="${arg.name}" data-arg-type="multi_list" multiple class="mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm" ${autofillAttr} ${requiredAttr}>${optionsHtml}</select></label><p class="text-xs text-gray-500 mt-1">Hold Ctrl/Cmd to select multiple options.</p>${wrapperEnd}`;
          } else if (arg.arg_type === "number") {
            html += `${wrapperStart}<label class="block">${labelSpan}<input type="number" id="${argId}" data-arg-name="${arg.name}" data-arg-type="number" value="${currentValue || ""}" class="mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm" ${autofillAttr} ${requiredAttr}></label>${wrapperEnd}`;
          } else if (arg.arg_type === "date_var") {
            const isVar = [
              "today",
              "tomorrow",
              "yesterday",
              "eomonth",
            ].includes(currentValue?.toLowerCase());
            const mode = isVar ? "var" : "calendar";

            const modeSelectId = `${argId}-mode`;

            html += `${wrapperStart}<label class="block">${labelSpan}
                <div class="flex items-center gap-2 mt-1">
                    <select id="${modeSelectId}" class="rounded border border-gray-300 px-2 py-2 text-sm" onchange="
                        const vSel = document.getElementById('${argId}-var');
                        const cInp = document.getElementById('${argId}-cal');
                        const mainInp = document.getElementById('${argId}');
                        if (this.value === 'var') {
                            vSel.classList.remove('hidden');
                            cInp.classList.add('hidden');
                            mainInp.value = vSel.value;
                        } else {
                            vSel.classList.add('hidden');
                            cInp.classList.remove('hidden');
                            mainInp.value = cInp.value;
                        }
                        mainInp.dispatchEvent(new Event('change'));
                    ">
                        <option value="calendar" ${mode === "calendar" ? "selected" : ""}>Calendar</option>
                        <option value="var" ${mode === "var" ? "selected" : ""}>Variable</option>
                    </select>

                    <select id="${argId}-var" class="w-full rounded border border-gray-300 px-3 py-2 text-sm ${mode === "var" ? "" : "hidden"}" onchange="document.getElementById('${argId}').value = this.value; document.getElementById('${argId}').dispatchEvent(new Event('change'));">
                        <option value="today" ${currentValue === "today" ? "selected" : ""}>today</option>
                        <option value="tomorrow" ${currentValue === "tomorrow" ? "selected" : ""}>tomorrow</option>
                        <option value="yesterday" ${currentValue === "yesterday" ? "selected" : ""}>yesterday</option>
                        <option value="eomonth" ${currentValue === "eomonth" ? "selected" : ""}>eomonth</option>
                    </select>

                    <input type="date" id="${argId}-cal" class="w-full rounded border border-gray-300 px-3 py-2 text-sm ${mode === "calendar" ? "" : "hidden"}" value="${!isVar ? currentValue || "" : ""}" onchange="document.getElementById('${argId}').value = this.value; document.getElementById('${argId}').dispatchEvent(new Event('change'));">

                    <input type="hidden" id="${argId}" data-arg-name="${arg.name}" data-arg-type="date_var" value="${currentValue || ""}" ${autofillAttr} ${requiredAttr}>
                </div>
            </label>${wrapperEnd}`;
          } else {
            // string
            html += `${wrapperStart}<label class="block">${labelSpan}<input type="text" id="${argId}" data-arg-name="${arg.name}" data-arg-type="string" value="${currentValue || ""}" class="mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm" ${autofillAttr} ${requiredAttr}></label>${wrapperEnd}`;
          }
        });
        html += "</div>";
      } else {
        html +=
          '<p class="text-sm text-gray-500 italic">This application takes no arguments.</p>';
      }

      externalAppDynamicInputs.innerHTML = html;

      // Setup dynamic visibility and autofill evaluation
      function evaluateDependencies(event) {
        const inputs = externalAppDynamicInputs.querySelectorAll(
          "input[data-arg-name], select[data-arg-name]",
        );
        const currentValues = {};
        inputs.forEach((input) => {
          if (input.type === "checkbox") {
            currentValues[input.getAttribute("data-arg-name")] = input.checked;
          } else {
            currentValues[input.getAttribute("data-arg-name")] = input.value;
          }
        });

        const wrappers =
          externalAppDynamicInputs.querySelectorAll(".arg-wrapper");
        wrappers.forEach((wrapper) => {
          const dependsStr = wrapper.getAttribute("data-depends-on");
          if (!dependsStr) return;

          try {
            const dependsOn = JSON.parse(dependsStr);
            let isVisible = true;
            for (const [depArgName, allowedValues] of Object.entries(
              dependsOn,
            )) {
              const val = currentValues[depArgName];
              if (val === undefined || !allowedValues.includes(val)) {
                isVisible = false;
                break;
              }
            }

            if (isVisible) {
              wrapper.classList.remove("hidden");
              // Re-enable required if it was required before
              const input = wrapper.querySelector(
                "input[data-arg-name], select[data-arg-name]",
              );
              if (input && input.hasAttribute("data-was-required")) {
                input.required = true;
              }
            } else {
              wrapper.classList.add("hidden");
              // Disable required so form can submit
              const input = wrapper.querySelector(
                "input[data-arg-name], select[data-arg-name]",
              );
              if (input && input.required) {
                input.setAttribute("data-was-required", "true");
                input.required = false;
              }
            }
          } catch (e) {
            console.error("Failed to parse depends_on", e);
          }
        });

        // Evaluate autofill only if an event triggered this (not on initial load)
        if (event && event.target) {
          const changedInputName = event.target.getAttribute("data-arg-name");
          if (!changedInputName) return;
          const changedValue = event.target.value;

          inputs.forEach((input) => {
            const autofillStr = input.getAttribute("data-autofill");
            if (!autofillStr) return;

            try {
              const autofills = JSON.parse(autofillStr);
              // autofills is a map of parent_arg -> {parent_val -> fill_val}
              if (autofills[changedInputName]) {
                const targetValue = autofills[changedInputName][changedValue];
                if (targetValue !== undefined) {
                  if (input.type === "checkbox") {
                    input.checked =
                      targetValue === "true" ||
                      targetValue === "on" ||
                      targetValue === true;
                  } else {
                    input.value = targetValue;
                  }
                } else {
                  // Revert to empty or default if parent value doesn't have a mapping
                  if (input.type === "checkbox") {
                    input.checked = false;
                  } else {
                    input.value = "";
                  }
                }
              }
            } catch (e) {
              console.error("Failed to parse autofill", e);
            }
          });
        }
      }

      const inputs = externalAppDynamicInputs.querySelectorAll(
        "input[data-arg-name], select[data-arg-name]",
      );
      inputs.forEach((input) => {
        input.addEventListener("change", evaluateDependencies);
        input.addEventListener("input", evaluateDependencies);
      });
      // Initial evaluation
      evaluateDependencies();
    } catch (e) {
      console.error("Failed to load app manifest", e);
      externalAppDynamicInputs.innerHTML =
        '<span class="text-sm text-red-500">Failed to load manifest. Ensure the app executable is correct.</span>';
    }
  }

  if (taskTypeSelect) {
    taskTypeSelect.addEventListener("change", updateTaskTypeVisibility);
    updateTaskTypeVisibility();
  }

  const form = document.querySelector("form");
  if (form) {
    form.addEventListener("submit", function (e) {
      if (schedulesHidden) schedulesHidden.value = buildSchedules();
      if (commandsHidden) commandsHidden.value = buildCommands();

      // Serialize external app args
      if (
        taskTypeSelect &&
        taskTypeSelect.value === "external_app" &&
        externalAppDynamicInputs
      ) {
        const argsMap = {};
        const inputs = externalAppDynamicInputs.querySelectorAll(
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
          alert(
            "Please fill in all required arguments for the external application.",
          );
          e.preventDefault();
          return false;
        }

        if (externalAppArgsHidden) {
          externalAppArgsHidden.value = JSON.stringify(argsMap);
        }
      }
    });
  }
})();
