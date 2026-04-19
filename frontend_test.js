const { chromium } = require('playwright');
const fs = require('fs');

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();

  await page.goto('http://127.0.0.1:8787/new-task');
  await page.waitForTimeout(1000);
  await page.screenshot({ path: 'frontend_task_type_crm.png' });

  // Select Shell Command
  await page.selectOption('#task-type-select', 'shell_command');
  await page.waitForTimeout(500);
  await page.screenshot({ path: 'frontend_task_type_shell.png' });

  // Add another command row
  await page.click('#add-command-row');
  await page.waitForTimeout(500);
  await page.screenshot({ path: 'frontend_task_type_shell_multi_command.png' });

  await browser.close();
})();
