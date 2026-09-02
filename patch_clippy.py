import re

with open("src/yasweb/browser/reports.rs", "r") as f:
    content = f.read()

bad_format = """                    let check_js_eval = format!(
                        "new Promise(resolve => {{
                            let attempts = 0;
                            let check = () => {{
                                let m = document.querySelector('.menuModules');
                                if (m && m.classList.contains('show-modules')) {{ resolve(true); }}
                                else if (attempts > 50) {{ resolve(false); }}
                                else {{ attempts++; setTimeout(check, 100); }}
                            }};
                            check();
                        }})"
                    );"""

good_format = """                    let check_js_eval = r#"
                        new Promise(resolve => {
                            let attempts = 0;
                            let check = () => {
                                let m = document.querySelector('.menuModules');
                                if (m && m.classList.contains('show-modules')) { resolve(true); }
                                else if (attempts > 50) { resolve(false); }
                                else { attempts++; setTimeout(check, 100); }
                            };
                            check();
                        })
                    "#;"""

content = content.replace(bad_format, good_format)

with open("src/yasweb/browser/reports.rs", "w") as f:
    f.write(content)
