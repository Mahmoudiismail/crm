with open("src/runner/gui/components.rs", "r") as f:
    content = f.read()

# Add a menu item for Working Hours Profiles
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

# Find the block and replace it
# But we need an icon for clock. We don't have icon_clock defined yet. Let's use icon_cube or similar for now or create icon_clock.
