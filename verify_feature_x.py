from playwright.sync_api import Page, expect, sync_playwright

def test_yasweb_manifest(page: Page):
  # Register yasweb app
  page.goto("http://127.0.0.1:8787/apps")
  page.locator("input[name='name']").fill("Yasweb")
  page.locator("input[name='id']").fill("yasweb")

  # create a dummy yasweb executable wrapper so the server can run it with --manifest
  page.locator("input[name='executable_path']").fill("/app/target/debug/yasweb")
  page.locator("input[name='config_path']").fill("/app/yasweb_config.json")
  page.locator("button[type='submit']").click()

  # Navigate to task creation to see the dynamic inputs
  page.goto("http://127.0.0.1:8787/new-task")

  # Select External App
  page.wait_for_selector("select[name='task_type']")
  page.locator("select[name='task_type']").select_option("external_app")

  # Wait for apps to load and select Yasweb
  page.wait_for_selector("#external-app-select")
  page.locator("#external-app-select").select_option("yasweb")

  # The manifest should render
  page.wait_for_selector("input[data-arg-name='--config']")

  # Select Report A -> --type should be autofilled with "type_a"
  page.locator("select[data-arg-name='--name']").select_option("Report A")
  expect(page.locator("input[data-arg-name='--type']")).to_have_value("type_a")

  page.screenshot(path="verification_report_a.png", full_page=True)

if __name__ == "__main__":
  with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    page = browser.new_page()
    try:
      test_yasweb_manifest(page)
    finally:
      browser.close()
