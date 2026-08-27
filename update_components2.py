with open("src/runner/gui/components.rs", "r") as f:
    content = f.read()

replacement = """            <nav class="mt-5 flex-1 px-2 space-y-1">
                <a href="/" class="text-gray-300 hover:bg-gray-700 hover:text-white group flex items-center px-2 py-2 text-sm font-medium rounded-md {dashboard_class}">
                    {icon_home}
                    Dashboard
                </a>
                <a href="/apps" class="text-gray-300 hover:bg-gray-700 hover:text-white group flex items-center px-2 py-2 text-sm font-medium rounded-md {apps_class}">
                    {icon_cube}
                    Applications
                </a>
                <a href="/working-hours" class="text-gray-300 hover:bg-gray-700 hover:text-white group flex items-center px-2 py-2 text-sm font-medium rounded-md {wh_class}">
                    {icon_clock}
                    Working Hours
                </a>
            </nav>"""

import re
content = re.sub(r'<nav class="mt-5 flex-1 px-2 space-y-1">[\s\S]*?</nav>', replacement, content)

content = content.replace(
    'let dashboard_class = if active_nav == "dashboard" { "bg-gray-900 text-white" } else { "" };',
    'let dashboard_class = if active_nav == "dashboard" { "bg-gray-900 text-white" } else { "" };\n    let wh_class = if active_nav == "system_status" { "bg-gray-900 text-white" } else { "" };'
)

# wait the active nav is apps vs dashboard vs system_status. Let's make sure format kwargs are correct
content = content.replace(
    'apps_class = apps_class,',
    'apps_class = apps_class,\n        wh_class = wh_class,\n        icon_clock = super::icons::icon_clock("mr-3 flex-shrink-0 h-6 w-6"),'
)

with open("src/runner/gui/components.rs", "w") as f:
    f.write(content)
