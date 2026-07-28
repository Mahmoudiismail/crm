(function () {
  const scheduleRows = document.getElementById("schedule-rows");
  const schedulesHidden = document.getElementById("schedules-hidden");
  const addScheduleBtn = document.getElementById("add-schedule-row");
  let scheduleIndex = scheduleRows ? scheduleRows.children.length : 0;

  function updateVisibility(row) {
    const kind = row.querySelector(".schedule-kind").value;
    row
      .querySelector(".schedule-interval")
      .classList.toggle("hidden", kind !== "interval");
    const scheduleSt = row.querySelector(".schedule-st");
    if (scheduleSt) {
      scheduleSt.classList.toggle("hidden", kind !== "interval" && kind !== "weekly" && kind !== "monthly");
    }
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

  function addScheduleRow() {
    const row = document.createElement("div");
    row.className = "flex flex-col gap-3 p-4 border border-gray-200 rounded-md bg-white";
    row.innerHTML = `
      <div class='flex flex-wrap items-end gap-3 w-full'>
          <div class='w-full sm:w-auto flex-1'>
              <label class='block text-xs font-medium text-gray-700 mb-1'>Type</label>
              <select class='schedule-kind shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2 bg-gray-50'>
                  <option value='interval'>Interval</option>
                  <option value='once'>Once</option>
                  <option value='daily'>Daily</option>
                  <option value='weekly'>Weekly</option>
                  <option value='monthly'>Monthly</option>
              </select>
          </div>
          <div class='schedule-interval w-full sm:w-auto flex-1'>
              <label class='block text-xs font-medium text-gray-700 mb-1'>Every</label>
              <select class='interval-value shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2'>
                  <option value='15m'>15m</option><option value='30m'>30m</option><option value='1h' selected>1h</option><option value='2h'>2h</option><option value='4h'>4h</option><option value='8h'>8h</option><option value='12h'>12h</option><option value='24h'>24h</option><option value='2d'>2d</option><option value='7d'>7d</option>
              </select>
          </div>
          <div class='schedule-once w-full sm:w-auto flex-1 hidden'>
              <label class='block text-xs font-medium text-gray-700 mb-1'>At</label>
              <input class='once-value shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2' type='datetime-local'>
          </div>
          <div class='schedule-daily w-full sm:w-auto flex-1 hidden'>
              <label class='block text-xs font-medium text-gray-700 mb-1'>Times (comma sep, e.g. 09:00,15:30)</label>
              <input class='daily-value shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2' type='text' placeholder='09:00'>
          </div>
          <div class='schedule-weekly w-full sm:w-auto flex-1 hidden'>
              <label class='block text-xs font-medium text-gray-700 mb-1'>Day and Time (e.g. Monday@09:00)</label>
              <input class='weekly-value shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2' type='text' placeholder='Monday@09:00'>
          </div>
          <div class='schedule-monthly w-full sm:w-auto flex-1 hidden'>
              <label class='block text-xs font-medium text-gray-700 mb-1'>Day and Time (e.g. 15@09:00 or -1@09:00)</label>
              <input class='monthly-value shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2' type='text' placeholder='1@09:00'>
          </div>
          <div class='schedule-st w-full sm:w-auto flex-1'>
            <label class='block text-xs font-medium text-gray-700 mb-1'>Start Time (Optional)</label>
            <input class='st-value shadow-sm focus:ring-emerald-500 focus:border-emerald-500 block w-full sm:text-sm border-gray-300 rounded-md border p-2' type='time'>
          </div>
          <div>
              <button type='button' class='remove-schedule inline-flex items-center p-2 border border-transparent rounded-md shadow-sm text-white bg-red-600 hover:bg-red-700 focus:outline-none'>
                  <svg class='h-4 w-4' fill='none' stroke='currentColor' viewBox='0 0 24 24'><path stroke-linecap='round' stroke-linejoin='round' stroke-width='2' d='M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16'></path></svg>
              </button>
          </div>
      </div>
      <div class='schedule-wh w-full bg-gray-50 p-3 rounded border border-gray-200'>
         <div class='flex items-center justify-between mb-2'>
             <span class='text-xs font-medium text-gray-700'>Working Hours (Optional, e.g. 09:00-17:00)</span>
         </div>
         <div class='grid grid-cols-2 sm:grid-cols-4 gap-2 text-xs'>
             <div><label class='block text-gray-600 mb-1'>Monday</label><input type='text' class='wh-mon block w-full rounded-md border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1.5' placeholder='09:00-17:00'></div>
             <div><label class='block text-gray-600 mb-1'>Tuesday</label><input type='text' class='wh-tue block w-full rounded-md border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1.5' placeholder='09:00-17:00'></div>
             <div><label class='block text-gray-600 mb-1'>Wednesday</label><input type='text' class='wh-wed block w-full rounded-md border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1.5' placeholder='09:00-17:00'></div>
             <div><label class='block text-gray-600 mb-1'>Thursday</label><input type='text' class='wh-thu block w-full rounded-md border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1.5' placeholder='09:00-17:00'></div>
             <div><label class='block text-gray-600 mb-1'>Friday</label><input type='text' class='wh-fri block w-full rounded-md border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1.5' placeholder='09:00-17:00'></div>
             <div><label class='block text-gray-600 mb-1'>Saturday</label><input type='text' class='wh-sat block w-full rounded-md border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1.5' placeholder='09:00-17:00'></div>
             <div><label class='block text-gray-600 mb-1'>Sunday</label><input type='text' class='wh-sun block w-full rounded-md border-gray-300 shadow-sm focus:ring-emerald-500 focus:border-emerald-500 p-1.5' placeholder='09:00-17:00'></div>
         </div>
      </div>
    `;

    const select = row.querySelector(".schedule-kind");
    select.addEventListener("change", () => updateVisibility(row));
    row.querySelector(".remove-schedule").addEventListener("click", () => {
      row.remove();
    });
    scheduleRows.appendChild(row);
    updateVisibility(row);
    scheduleIndex++;
  }

  function getWorkingHours(row) {
    const wh = {};
    const days = [
      { key: "Monday", cls: ".wh-mon" },
      { key: "Tuesday", cls: ".wh-tue" },
      { key: "Wednesday", cls: ".wh-wed" },
      { key: "Thursday", cls: ".wh-thu" },
      { key: "Friday", cls: ".wh-fri" },
      { key: "Saturday", cls: ".wh-sat" },
      { key: "Sunday", cls: ".wh-sun" }
    ];
    let hasWh = false;
    for (const day of days) {
      const val = row.querySelector(day.cls).value.trim();
      if (val) {
        const parts = val.split('-');
        if (parts.length === 2) {
          wh[day.key] = { start: parts[0].trim(), end: parts[1].trim() };
          hasWh = true;
        }
      }
    }
    return hasWh ? wh : null;
  }

  function buildSchedules() {
    if (!scheduleRows) return "";
    const schedules = [];
    const rows = scheduleRows.querySelectorAll(".flex-col");
    for (const row of rows) {
      const kind = row.querySelector(".schedule-kind").value;
      const stVal = row.querySelector(".st-value").value.trim();
      let schedule = null;
      if (kind === "interval") {
        schedule = `interval:${row.querySelector(".interval-value").value}`;
      } else if (kind === "once") {
        schedule = `once:${row.querySelector(".once-value").value}`;
      } else if (kind === "daily") {
        schedule = `daily:${row.querySelector(".daily-value").value}`;
      } else if (kind === "weekly") {
        schedule = `weekly:${row.querySelector(".weekly-value").value}`;
      } else if (kind === "monthly") {
        schedule = `monthly:${row.querySelector(".monthly-value").value}`;
      }

      let whStr = "";
      if (kind === "interval" || kind === "daily") {
          const wh = getWorkingHours(row);
          if (wh) {
              const parts = [];
              for (const [day, range] of Object.entries(wh)) {
                  parts.push(`${day}=${range.start}-${range.end}`);
              }
              whStr = "; wh: " + parts.join(",");
          }
      }
      if (stVal && (kind === "interval" || kind === "weekly" || kind === "monthly")) {
        schedule = schedule + "; st: " + stVal + whStr;
      } else {
        schedule = schedule + whStr;
      }
      schedules.push(schedule);
    }
    return schedules.join("\n");
  }

  if (addScheduleBtn) {
    addScheduleBtn.addEventListener("click", addScheduleRow);
  }

  if (scheduleRows) {
    for (const row of scheduleRows.querySelectorAll(".flex-col")) {
      const select = row.querySelector(".schedule-kind");
      select.addEventListener("change", () => updateVisibility(row));
      row.querySelector(".remove-schedule").addEventListener("click", () => {
        row.remove();
      });
      updateVisibility(row);
    }
  }


  // Multi-step form logic

  function renderActionEditor(action, actionContainer, appIdBase) {
      actionContainer.innerHTML = '';
      const actionType = action ? action.type : "shell_command";

      const typeSelect = document.createElement("select");
      typeSelect.className = "action-type-select mb-3 block w-full rounded border border-gray-300 px-3 py-2 text-sm";
      typeSelect.innerHTML = `
          <option value="shell_command" ${actionType === 'shell_command' ? 'selected' : ''}>Shell Command</option>
          <option value="external_app" ${actionType === 'external_app' ? 'selected' : ''}>External Application</option>
      `;
      actionContainer.appendChild(typeSelect);

      const contentDiv = document.createElement("div");
      contentDiv.className = "action-content";
      actionContainer.appendChild(contentDiv);

      function updateContent() {
          const type = typeSelect.value;
          if (type === "shell_command") {
              const cmd = (action && action.type === "shell_command") ? action.command : "";
              const c_err = (action && action.type === "shell_command") ? action.continue_on_error : false;
              contentDiv.innerHTML = `
                  <div class="space-y-3">
                      <div>
                          <label class="block text-xs font-medium text-gray-700 mb-1">Command</label>
                          <input type="text" class="action-cmd block w-full rounded border border-gray-300 px-3 py-2 text-sm" value="${cmd.replace(/"/g, '&quot;')}" placeholder="echo hello">
                      </div>
                      <div>
                          <label class="block text-xs font-medium text-gray-700 mb-1">Continue on Error</label>
                          <select class="action-cerr block w-full rounded border border-gray-300 px-3 py-2 text-sm">
                              <option value="false" ${!c_err ? 'selected' : ''}>No</option>
                              <option value="true" ${c_err ? 'selected' : ''}>Yes</option>
                          </select>
                      </div>
                  </div>
              `;
          } else {
              const appId = (action && action.type === "external_app") ? action.app_id : "";
              const appArgs = (action && action.type === "external_app") ? action.args : {};
              const argsJson = JSON.stringify(appArgs).replace(/"/g, '&quot;');

              contentDiv.innerHTML = `
                  <div class="space-y-3">
                      <div class="app-select-container mb-4"></div>
                      <div class="app-dynamic-inputs space-y-3"></div>
                      <input type="hidden" class="app-args-hidden" value="${argsJson}">
                      <input type="hidden" class="app-id-hidden" value="${appId}">
                  </div>
              `;

              const selContainer = contentDiv.querySelector(".app-select-container");
              const dynContainer = contentDiv.querySelector(".app-dynamic-inputs");
              const idHidden = contentDiv.querySelector(".app-id-hidden");
              const argsHidden = contentDiv.querySelector(".app-args-hidden");

              if (window.api.getRegisteredAppsCache() === null || window.api.getRegisteredAppsCache() === undefined) {
                  loadRegisteredAppsForContainer(selContainer, dynContainer, idHidden, argsHidden, appIdBase);
              } else {
                  renderAppList(window.api.getRegisteredAppsCache(), selContainer, dynContainer, idHidden, argsHidden, appIdBase);
              }
          }
      }

      typeSelect.addEventListener("change", updateContent);
      updateContent();
  }

  function addStepBlock(container, step, index, isPostRun) {
      const stepDiv = document.createElement("div");
      stepDiv.className = "step-block p-4 border border-gray-300 bg-white rounded shadow-sm relative";

      const stepName = step ? (step.name || "") : (isPostRun ? "Post Run Step" : "Step");
      const stepMode = step ? step.mode : "sequential";
      const actions = step ? (step.actions || []) : [];

      stepDiv.innerHTML = `
          <button type="button" class="remove-step absolute top-2 right-2 text-red-500 hover:text-red-700 font-bold px-2 py-1 bg-red-50 rounded">X</button>
          <div class="mb-4">
              <label class="block text-xs font-medium text-gray-700 mb-1">Step Name (Optional)</label>
              <input type="text" class="step-name block w-full rounded border border-gray-300 px-3 py-2 text-sm" value="${stepName.replace(/"/g, '&quot;')}" placeholder="My Step">
          </div>
          <div class="mb-4">
              <label class="block text-xs font-medium text-gray-700 mb-1">Execution Mode</label>
              <select class="step-mode block w-full rounded border border-gray-300 px-3 py-2 text-sm">
                  <option value="sequential" ${stepMode === 'sequential' ? 'selected' : ''}>Sequential</option>
                  <option value="parallel" ${stepMode === 'parallel' ? 'selected' : ''}>Parallel</option>
              </select>
          </div>

          <div class="actions-container space-y-4"></div>
          <button type="button" class="add-action-btn mt-3 rounded border border-gray-300 bg-gray-100 text-gray-800 px-3 py-1 text-xs font-semibold hover:bg-gray-200">+ Add Action</button>
      `;

      container.appendChild(stepDiv);

      const actionsContainer = stepDiv.querySelector(".actions-container");

      function renderAction(act) {
          const actionDiv = document.createElement("div");
          actionDiv.className = "action-block p-3 border border-purple-200 bg-purple-50 rounded relative";
          actionDiv.innerHTML = `<button type="button" class="remove-action absolute top-2 right-2 text-red-500 hover:text-red-700 font-bold px-2 py-1 text-xs rounded">X</button><div class="action-editor"></div>`;
          actionsContainer.appendChild(actionDiv);

          actionDiv.querySelector(".remove-action").addEventListener("click", () => actionDiv.remove());

          const editorDiv = actionDiv.querySelector(".action-editor");
          const appIdBase = "app-" + Date.now() + Math.random().toString(36).substr(2, 5);
          renderActionEditor(act, editorDiv, appIdBase);
      }

      if (actions.length === 0) {
          renderAction(null);
      } else {
          actions.forEach(a => renderAction(a));
      }

      stepDiv.querySelector(".add-action-btn").addEventListener("click", () => {
          renderAction(null);
      });

      stepDiv.querySelector(".remove-step").addEventListener("click", () => {
          stepDiv.remove();
      });
  }

  function initSteps(containerId, hiddenId, isPostRun) {
      const container = document.getElementById(containerId);
      const hidden = document.getElementById(hiddenId);
      const addBtn = document.getElementById(isPostRun ? "add-post-run-step-btn" : "add-step-btn");

      if (!container || !hidden) return;

      let initialData = [];
      try {
          initialData = JSON.parse(hidden.value || "[]");
      } catch (e) {
          console.error("Failed to parse steps JSON", e);
      }

      initialData.forEach((s, i) => addStepBlock(container, s, i, isPostRun));

      addBtn.addEventListener("click", () => {
          addStepBlock(container, null, container.children.length, isPostRun);
      });
  }

  function serializeSteps(containerId) {
      const container = document.getElementById(containerId);
      if (!container) return [];

      const steps = [];
      const stepBlocks = container.querySelectorAll(".step-block");

      stepBlocks.forEach(stepBlock => {
          const stepName = stepBlock.querySelector(".step-name").value.trim();
          const stepMode = stepBlock.querySelector(".step-mode").value;

          const actions = [];
          const actionBlocks = stepBlock.querySelectorAll(".action-block");

          actionBlocks.forEach(actionBlock => {
              const type = actionBlock.querySelector(".action-type-select").value;
              if (type === "shell_command") {
                  const cmd = actionBlock.querySelector(".action-cmd").value.trim();
                  const cerr = actionBlock.querySelector(".action-cerr").value === "true";
                  if (cmd) {
                      actions.push({
                          type: "shell_command",
                          command: cmd,
                          continue_on_error: cerr
                      });
                  }
              } else if (type === "external_app") {
                  const dynContainer = actionBlock.querySelector(".app-dynamic-inputs");
                  const argsHidden = actionBlock.querySelector(".app-args-hidden");

                  if (window.validation.serializeExternalApp(dynContainer, argsHidden)) {
                      const appId = actionBlock.querySelector(".app-id-hidden").value;
                      let args = {};
                      try { args = JSON.parse(argsHidden.value); } catch(e){}
                      if (appId) {
                          actions.push({
                              type: "external_app",
                              app_id: appId,
                              args: args
                          });
                      }
                  } else {
                     throw new Error("Validation failed for external app");
                  }
              }
          });

          steps.push({
              name: stepName ? stepName : null,
              mode: stepMode,
              actions: actions
          });
      });
      return steps;
  }


  function loadRegisteredAppsForContainer(selectContainer, dynamicInputs, idHidden, argsHidden, idPrefix) {
      window.api
        .fetchApps()
        .then((apps) => {
          window.api.setRegisteredAppsCache(apps);
          renderAppList(apps, selectContainer, dynamicInputs, idHidden, argsHidden, idPrefix);
        })
        .catch((err) => {
          console.error("Failed to load apps", err);
          selectContainer.innerHTML =
            '<span class="text-sm text-red-500">Failed to load registered applications.</span>';
        });
  }

  function renderAppList(apps, selectContainer, dynamicInputs, idHidden, argsHidden, idPrefix) {
    if (apps.length === 0) {
      selectContainer.innerHTML =
        '<span class="text-sm text-gray-500">No applications registered. Register one in the Apps section.</span>';
      return;
    }

    let options = "<option value=''>-- Select Application --</option>";
    const selectedId = idHidden.value;
    apps.forEach((app) => {
      const selected = app.id === selectedId ? "selected" : "";
      options += `<option value="${app.id}" ${selected}>${app.name} (${app.id})</option>`;
    });

    selectContainer.innerHTML = `
          <label class="block mb-2">
              <span class="text-sm font-semibold text-gray-700">Application</span>
              <select class="mt-1 block w-full rounded border border-gray-300 px-3 py-2 text-sm bg-white" id="${idPrefix}-select">
                  ${options}
              </select>
          </label>
      `;

    const selectEl = document.getElementById(`${idPrefix}-select`);
    selectEl.addEventListener("change", function () {
      idHidden.value = this.value;
      if (this.value) {
        argsHidden.value = "{}"; // Reset args when app changes
        loadAppManifest(this.value, dynamicInputs, argsHidden, idPrefix);
      } else {
        dynamicInputs.innerHTML = "";
        argsHidden.value = "{}";
      }
    });

    if (selectedId) {
      loadAppManifest(selectedId, dynamicInputs, argsHidden, idPrefix);
    }
  }

  async function loadAppManifest(appId, dynamicInputs, argsHidden, idPrefix) {
    dynamicInputs.innerHTML =
      '<div class="text-sm text-gray-500">Loading manifest...</div>';
    try {
      const manifest = await window.api.fetchAppManifest(appId);
      if (!manifest || !manifest.arguments || manifest.arguments.length === 0) {
        dynamicInputs.innerHTML =
          '<span class="text-sm text-gray-500">No configurable arguments.</span>';
        return;
      }

      let currentArgs = {};
      try {
        currentArgs = JSON.parse(argsHidden.value || "{}");
      } catch (e) {
        console.error("Failed to parse existing args", e);
      }

      let html =
        '<h4 class="text-sm font-semibold text-gray-800 border-b border-gray-200 pb-1 mb-3">Arguments</h4>';

      manifest.arguments.forEach((arg) => {
        const val =
          currentArgs[arg.name] !== undefined
            ? currentArgs[arg.name]
            : arg.default_value;
        const requiredAsterisk = arg.required
          ? '<span class="text-red-500">*</span>'
          : "";
        const id = `${idPrefix}-arg-${arg.name}`;

        let dependsData = "";
        if (arg.depends_on) {
          dependsData = ` data-depends-on="${arg.depends_on.arg_name}" data-depends-val="${arg.depends_on.expected_value}"`;
        }

        if (arg.arg_type === "Boolean") {
          const checked = val === "true" || val === true ? "checked" : "";
          html += `
            <div class="mb-3 arg-container" ${dependsData}>
                <label class="flex items-center gap-2 text-sm text-gray-700">
                    <input type="checkbox" id="${id}" data-arg-name="${arg.name}" ${checked} class="rounded border-gray-300 text-emerald-600 focus:ring-emerald-500">
                    <span class="font-semibold">${arg.name}</span> ${requiredAsterisk}
                </label>
                ${arg.description ? `<p class="text-xs text-gray-500 mt-1 ml-6">${arg.description}</p>` : ""}
            </div>
          `;
        } else if (
          arg.arg_type === "String" &&
          arg.allowed_values &&
          arg.allowed_values.length > 0
        ) {
          let opts = "";
          arg.allowed_values.forEach((opt) => {
            const selected = opt === val ? "selected" : "";
            opts += `<option value="${opt}" ${selected}>${opt}</option>`;
          });
          html += `
            <div class="mb-3 arg-container" ${dependsData}>
                <label class="block text-sm font-semibold text-gray-700 mb-1">${arg.name} ${requiredAsterisk}</label>
                <select id="${id}" data-arg-name="${arg.name}" class="block w-full rounded border border-gray-300 px-3 py-2 text-sm">
                    ${opts}
                </select>
                ${arg.description ? `<p class="text-xs text-gray-500 mt-1">${arg.description}</p>` : ""}
            </div>
          `;
        } else if (arg.arg_type === "DateVar") {
             const dateVars = ["today", "yesterday", "tomorrow", "eomonth"];
             let isVar = dateVars.includes(val);
             let dateVal = isVar ? "" : val;
             let varVal = isVar ? val : "today";

             html += `
               <div class="mb-3 arg-container" ${dependsData}>
                   <label class="block text-sm font-semibold text-gray-700 mb-1">${arg.name} ${requiredAsterisk}</label>
                   <div class="flex gap-2 mb-1">
                       <label class="text-xs flex items-center gap-1"><input type="radio" name="mode_${id}" value="fixed" ${!isVar ? 'checked' : ''} onchange="document.getElementById('fixed_${id}').style.display='block'; document.getElementById('var_${id}').style.display='none';"> Fixed Date</label>
                       <label class="text-xs flex items-center gap-1"><input type="radio" name="mode_${id}" value="var" ${isVar ? 'checked' : ''} onchange="document.getElementById('fixed_${id}').style.display='none'; document.getElementById('var_${id}').style.display='block';"> Dynamic</label>
                   </div>
                   <input type="date" id="fixed_${id}" data-arg-name="${arg.name}" data-group="datevar" class="block w-full rounded border border-gray-300 px-3 py-2 text-sm" value="${dateVal}" style="display: ${!isVar ? 'block' : 'none'}">
                   <select id="var_${id}" data-arg-name="${arg.name}" data-group="datevar" class="block w-full rounded border border-gray-300 px-3 py-2 text-sm" style="display: ${isVar ? 'block' : 'none'}">
                      <option value="today" ${varVal === 'today' ? 'selected' : ''}>Today</option>
                      <option value="yesterday" ${varVal === 'yesterday' ? 'selected' : ''}>Yesterday</option>
                      <option value="tomorrow" ${varVal === 'tomorrow' ? 'selected' : ''}>Tomorrow</option>
                      <option value="eomonth" ${varVal === 'eomonth' ? 'selected' : ''}>End of Month</option>
                   </select>
                   ${arg.description ? `<p class="text-xs text-gray-500 mt-1">${arg.description}</p>` : ""}
               </div>
             `;
        } else if (arg.arg_type === "MultiList" && arg.allowed_values && arg.allowed_values.length > 0) {
            let currentList = [];
            if (val) {
                currentList = val.split(',').map(s => s.trim());
            }
            let checkBoxes = "";
            arg.allowed_values.forEach((opt, idx) => {
               const checked = currentList.includes(opt) ? "checked" : "";
               checkBoxes += `
                 <label class="flex items-center gap-2 text-sm text-gray-700 py-1">
                     <input type="checkbox" data-multilist="${arg.name}" value="${opt}" ${checked} class="rounded border-gray-300 text-emerald-600 focus:ring-emerald-500">
                     <span>${opt}</span>
                 </label>
               `;
            });

            html += `
              <div class="mb-3 arg-container" ${dependsData}>
                  <label class="block text-sm font-semibold text-gray-700 mb-1">${arg.name} ${requiredAsterisk}</label>
                  <div class="max-h-48 overflow-y-auto border border-gray-300 rounded p-2 bg-white" data-arg-name="${arg.name}" data-group="multilist" id="${id}">
                      ${checkBoxes}
                  </div>
                  ${arg.description ? `<p class="text-xs text-gray-500 mt-1">${arg.description}</p>` : ""}
              </div>
            `;

        } else {
          html += `
            <div class="mb-3 arg-container" ${dependsData}>
                <label class="block text-sm font-semibold text-gray-700 mb-1">${arg.name} ${requiredAsterisk}</label>
                <input type="text" id="${id}" data-arg-name="${arg.name}" class="block w-full rounded border border-gray-300 px-3 py-2 text-sm" value="${(val || "").replace(/"/g, '&quot;')}">
                ${arg.description ? `<p class="text-xs text-gray-500 mt-1">${arg.description}</p>` : ""}
            </div>
          `;
        }
      });

      dynamicInputs.innerHTML = html;

      function evaluateDependencies() {
        const containers = dynamicInputs.querySelectorAll(".arg-container[data-depends-on]");
        containers.forEach(container => {
           const depName = container.getAttribute("data-depends-on");
           const depVal = container.getAttribute("data-depends-val");

           const depInput = dynamicInputs.querySelector(`[data-arg-name="${depName}"]`);
           if (depInput) {
               let currentVal = "";
               if (depInput.type === "checkbox" && !depInput.hasAttribute("data-multilist")) {
                   currentVal = depInput.checked ? "true" : "false";
               } else if (depInput.tagName === "SELECT") {
                   currentVal = depInput.value;
               } else {
                   currentVal = depInput.value;
               }

               if (currentVal === depVal) {
                   container.style.display = "block";
               } else {
                   container.style.display = "none";
               }
           }
        });
      }

      const inputs = dynamicInputs.querySelectorAll(
        "input[data-arg-name], select[data-arg-name], input[type='radio']",
      );
      inputs.forEach((input) => {
        input.addEventListener("change", evaluateDependencies);
        input.addEventListener("input", evaluateDependencies);
      });
      evaluateDependencies();
    } catch (e) {
      console.error("Failed to load app manifest", e);
      dynamicInputs.innerHTML =
        '<span class="text-sm text-red-500">Failed to load manifest. Ensure the app executable is correct.</span>';
    }
  }


  initSteps("steps-container", "steps-hidden", false);
  initSteps("post-run-steps-container", "post-run-steps-hidden", true);

  const form = document.querySelector("form");
  if (form) {
    form.addEventListener("submit", function (e) {
      if (schedulesHidden) schedulesHidden.value = buildSchedules();

      try {
          const steps = serializeSteps("steps-container");
          document.getElementById("steps-hidden").value = JSON.stringify(steps);

          const postRunSteps = serializeSteps("post-run-steps-container");
          document.getElementById("post-run-steps-hidden").value = JSON.stringify(postRunSteps);
      } catch (err) {
          alert(err.message || "Please fill in all required arguments.");
          e.preventDefault();
          return false;
      }
    });
  }
})();
