(function() {
    const scheduleRows = document.getElementById('schedule-rows');
    const commandRows = document.getElementById('command-rows');
    const schedulesHidden = document.getElementById('schedules-hidden');
    const commandsHidden = document.getElementById('commands-hidden');
    const addScheduleBtn = document.getElementById('add-schedule-row');
    const addCommandBtn = document.getElementById('add-command-row');
    let scheduleIndex = scheduleRows ? scheduleRows.children.length : 0;
    let commandIndex = commandRows ? commandRows.children.length : 0;

    function updateVisibility(row) {
        const kind = row.querySelector('.schedule-kind').value;
        row.querySelector('.schedule-interval').classList.toggle('hidden', kind !== 'interval');
        row.querySelector('.schedule-once').classList.toggle('hidden', kind !== 'once');
        row.querySelector('.schedule-daily').classList.toggle('hidden', kind !== 'daily');
        row.querySelector('.schedule-weekly').classList.toggle('hidden', kind !== 'weekly');
        row.querySelector('.schedule-monthly').classList.toggle('hidden', kind !== 'monthly');
        const whContainer = row.querySelector('.schedule-wh');
        if (whContainer) {
            whContainer.classList.toggle('hidden', kind !== 'interval' && kind !== 'daily');
        }
    }

    function createWhRow() {
        const row = document.createElement('div');
        row.className = 'flex gap-2 items-center mt-2';
        row.setAttribute('data-wh-row', '');
        const daysOfWeek = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
        row.innerHTML = `
            <select class='rounded border border-gray-300 px-2 py-1 text-sm wh-day'>
                ${daysOfWeek.map(day => `<option value='${day}'>${day}</option>`).join('')}
            </select>
            <input class='rounded border border-gray-300 px-2 py-1 text-sm w-24 wh-start' type='time' value='09:00'>
            <span class='text-xs text-gray-500'>to</span>
            <input class='rounded border border-gray-300 px-2 py-1 text-sm w-24 wh-end' type='time' value='17:00'>
            <button type='button' class='remove-wh rounded bg-red-100 px-2 py-1 text-xs font-semibold text-red-700'>&times;</button>
        `;
        return row;
    }

    function attachWhEvents(row) {
        const removeBtn = row.querySelector('.remove-wh');
        if (removeBtn) {
            removeBtn.addEventListener('click', function() {
                row.remove();
            });
        }
    }

    function attachScheduleEvents(row) {
        row.querySelector('.schedule-kind').addEventListener('change', function() {
            updateVisibility(row);
        });
        row.querySelector('.remove-schedule').addEventListener('click', function() {
            row.remove();
        });
        const addWhBtn = row.querySelector('.add-wh-row');
        if (addWhBtn) {
            addWhBtn.addEventListener('click', function() {
                const whRows = row.querySelector('.wh-rows');
                const whRow = createWhRow();
                whRows.appendChild(whRow);
                attachWhEvents(whRow);
            });
        }
        Array.from(row.querySelectorAll('[data-wh-row]')).forEach(attachWhEvents);
    }

    function attachCommandEvents(row) {
        row.querySelector('.remove-command').addEventListener('click', function() {
            row.remove();
        });
    }

    function createScheduleRow(kind, interval, once, daily, weekly, monthly) {
        const row = document.createElement('div');
        row.setAttribute('data-schedule-row', '');
        row.className = 'grid md:grid-cols-5 gap-2 p-3 border border-gray-200 rounded items-end';
        const daysOfWeek = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
        row.innerHTML = `
            <label class='block'>
                <span class='text-xs font-semibold text-gray-700'>Type</span>
                <select class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm schedule-kind'>
                    <option value='interval' ${kind === 'interval' ? 'selected' : ''}>Interval</option>
                    <option value='once' ${kind === 'once' ? 'selected' : ''}>Once</option>
                    <option value='daily' ${kind === 'daily' ? 'selected' : ''}>Daily</option>
                    <option value='weekly' ${kind === 'weekly' ? 'selected' : ''}>Weekly</option>
                    <option value='monthly' ${kind === 'monthly' ? 'selected' : ''}>Monthly</option>
                </select>
            </label>
            <label class='block schedule-interval ${kind === 'interval' ? '' : 'hidden'}'>
                <span class='text-xs font-semibold text-gray-700'>Interval</span>
                <select class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm'>
                    ${['15m','30m','1h','2h','4h','8h','12h','24h','2d','7d'].map(opt => `<option value='${opt}' ${opt === interval ? 'selected' : ''}>${opt}</option>`).join('')}
                </select>
            </label>
            <label class='block schedule-once ${kind === 'once' ? '' : 'hidden'}'>
                <span class='text-xs font-semibold text-gray-700'>Date & Time</span>
                <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='datetime-local' value='${once}'>
            </label>
            <label class='block schedule-daily ${kind === 'daily' ? '' : 'hidden'}'>
                <span class='text-xs font-semibold text-gray-700'>Times (HH:MM)</span>
                <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='text' value='${daily}' placeholder='09:00, 13:00'>
            </label>
            <label class='block schedule-weekly ${kind === 'weekly' ? '' : 'hidden'}'>
                <span class='text-xs font-semibold text-gray-700'>Day</span>
                <select class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' data-weekly-day>
                    ${daysOfWeek.map(day => `<option value='${day}' ${day === weekly ? 'selected' : ''}>${day}</option>`).join('')}
                </select>
            </label>
            <label class='block schedule-monthly ${kind === 'monthly' ? '' : 'hidden'}'>
                <span class='text-xs font-semibold text-gray-700'>Day (1-31)</span>
                <input class='mt-1 w-full rounded border border-gray-300 px-3 py-2 text-sm' type='number' value='${monthly}' min='1' max='31'>
            </label>
            <button type='button' class='remove-schedule rounded border border-red-200 bg-red-50 px-3 py-2 text-sm font-semibold text-red-700'>Remove</button>
          </div>
          <div class='mt-3 schedule-wh ${kind === 'interval' || kind === 'daily' ? '' : 'hidden'}'>
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
        const row = document.createElement('div');
        row.setAttribute('data-command-row', '');
        row.className = 'grid md:grid-cols-[1fr_100px_auto] gap-2 items-center p-2 bg-gray-50 border border-gray-200 rounded';
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
        Array.from(scheduleRows.querySelectorAll('[data-schedule-row]')).forEach(attachScheduleEvents);
    }
    if (commandRows) {
        Array.from(commandRows.querySelectorAll('[data-command-row]')).forEach(attachCommandEvents);
    }
    if (addScheduleBtn) {
        addScheduleBtn.addEventListener('click', function() {
            const scheduleRows = document.getElementById('schedule-rows');
            if (!scheduleRows) return;
            const row = createScheduleRow('interval', '1h', '', '', '', '');
            scheduleRows.appendChild(row);
            attachScheduleEvents(row);
            scheduleIndex += 1;
        });
    }
    if (addCommandBtn) {
        addCommandBtn.addEventListener('click', function() {
            const commandRows = document.getElementById('command-rows');
            if (!commandRows) return;
            const row = createCommandRow('');
            commandRows.appendChild(row);
            attachCommandEvents(row);
            commandIndex += 1;
        });
    }

    function encodeIsoDatetime(value) {
        if (!value) return '';
        const date = new Date(value);
        return date.toISOString();
    }

    function buildSchedules() {
        if (!scheduleRows) return '';
        return Array.from(scheduleRows.querySelectorAll('[data-schedule-row]')).map(function(row) {
            const kind = row.querySelector('.schedule-kind').value;
            if (kind === 'interval') {
                const interval = row.querySelector('.schedule-interval select').value;
                const whRows = Array.from(row.querySelectorAll('[data-wh-row]')).map(whRow => {
                    const day = whRow.querySelector('.wh-day').value;
                    const start = whRow.querySelector('.wh-start').value;
                    const end = whRow.querySelector('.wh-end').value;
                    if (day && start && end) {
                        return `${day}=${start}-${end}`;
                    }
                    return null;
                }).filter(Boolean).join(',');

                if (whRows) {
                    return 'interval: every ' + interval + '; wh: ' + whRows;
                } else {
                    return 'interval: every ' + interval;
                }
            }
            if (kind === 'once') {
                const value = row.querySelector('.schedule-once input').value;
                return value ? 'once: ' + encodeIsoDatetime(value) : 'once:';
            }
            if (kind === 'daily') {
                const value = row.querySelector('.schedule-daily input').value.trim();
                const whRows = Array.from(row.querySelectorAll('[data-wh-row]')).map(whRow => {
                    const day = whRow.querySelector('.wh-day').value;
                    const start = whRow.querySelector('.wh-start').value;
                    const end = whRow.querySelector('.wh-end').value;
                    if (day && start && end) {
                        return `${day}=${start}-${end}`;
                    }
                    return null;
                }).filter(Boolean).join(',');

                if (whRows) {
                    return value ? 'daily: ' + value + '; wh: ' + whRows : '';
                } else {
                    return value ? 'daily: ' + value : '';
                }
            }
            if (kind === 'weekly') {
                const value = row.querySelector('[data-weekly-day]').value;
                return value ? 'weekly: ' + value : '';
            }
            if (kind === 'monthly') {
                const value = row.querySelector('.schedule-monthly input').value;
                return value ? 'monthly: day ' + value : '';
            }
            return '';
        }).filter(Boolean).join('\n');
    }

    function buildCommands() {
        if (!commandRows) return '';
        return Array.from(commandRows.querySelectorAll('[data-command-row]')).map(function(row) {
            const command = row.querySelector('.command-text').value.trim();
            const mode = row.querySelector('.command-mode').value;
            if (!command) return '';
            return mode === 'continue' ? 'continue: ' + command : command;
        }).filter(Boolean).join('\n');
    }

    const taskTypeSelect = document.getElementById('task-type-select');
    const crmReportContainer = document.getElementById('crm-report-container');
    const shellCommandContainer = document.getElementById('shell-command-container');
    const yaswebContainer = document.getElementById('yasweb-container');

    function updateTaskTypeVisibility() {
        if (!taskTypeSelect) return;
        const type = taskTypeSelect.value;
        if (crmReportContainer) {
            crmReportContainer.classList.toggle('hidden', type !== 'crm_fetch');
        }
        if (shellCommandContainer) {
            shellCommandContainer.classList.toggle('hidden', type !== 'shell_command');
        }
        if (yaswebContainer) {
            yaswebContainer.classList.toggle('hidden', type !== 'yasweb');
        }
    }

    if (taskTypeSelect) {
        taskTypeSelect.addEventListener('change', updateTaskTypeVisibility);
        updateTaskTypeVisibility();
    }

    const form = document.querySelector('form');
    if (form) {
        form.addEventListener('submit', function() {
            schedulesHidden.value = buildSchedules();
            commandsHidden.value = buildCommands();
        });
    }
})();
