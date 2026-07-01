with open("md/ARCHITECTURE.md", "r") as f:
    content = f.read()

import re
content = re.sub(r'The `runner` engine supports different task types: `CrmFetch`, `ShellCommand`, `Yasweb`, and `ExternalApp`\.', 'The `runner` engine supports different task types: `ShellCommand` and `ExternalApp`.', content)
content = re.sub(r'- For `CrmFetch`, it delegates execution to the `crm` executable.*?\n', '', content)
content = re.sub(r'- For `Yasweb`, it delegates execution to the `yasweb` executable.*?\n', '', content)

with open("md/ARCHITECTURE.md", "w") as f:
    f.write(content)

with open("md/SCHEDULER_TRAY.md", "r") as f:
    content = f.read()

content = re.sub(r'It features four primary task types:.*?\n\s*-.*?\n\s*-.*?\n\s*-.*?\n\s*-.*?\n', 'It features two primary task types:\n- **Shell Command:** Executes sequential or parallel batch/powershell/bash scripts.\n- **External App:** Dynamically registers executables (like `crm.exe` or `yasweb.exe`) and generates GUI forms from their JSON `AppManifest` via the `--manifest` flag.\n\n', content, flags=re.DOTALL)
content = re.sub(r'The tray GUI provides specific forms for building Yasweb and CRM tasks.*?`ExternalApp`.*?tasks\.', 'The tray GUI provides a form for building `ExternalApp` or `ShellCommand` tasks.', content)

with open("md/SCHEDULER_TRAY.md", "w") as f:
    f.write(content)
