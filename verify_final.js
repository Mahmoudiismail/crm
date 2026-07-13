const { chromium } = require('playwright');
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  try {
    await page.goto('http://localhost:3000/apps/new');
    // Check for Start Time field
    await page.selectOption('select[name="schedule_type"]', 'Interval');
    await page.waitForSelector('input[name="interval_start_time"]', { state: 'visible' });
    await page.screenshot({ path: 'final_form_interval.png', fullPage: true });

    // Check two-column layout for Post Run Script and Timeout
    const postRunLabel = await page.locator('span:has-text("Post Run Script")').parentElement();
    const timeoutLabel = await page.locator('span:has-text("Timeout (Seconds)")').parentElement();
    console.log('Post Run Script parent classes:', await postRunLabel.getAttribute('class'));
    console.log('Timeout parent classes:', await timeoutLabel.getAttribute('class'));

  } catch (e) {
    console.error(e);
  } finally {
    await browser.close();
  }
})();
