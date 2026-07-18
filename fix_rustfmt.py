with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

import re

search_pattern = r"""            let owner = team_to_owner.get\(&team_lower\);
            let email = team_to_email.get\(&team_lower\);

            let oul = match \(owner, email\) \{"""

replace_pattern = r"""            let owner = team_to_owner.get(&team_lower);
            let email = team_to_email.get(&team_lower);

            let oul = match (owner, email) {"""

content = re.sub(search_pattern, replace_pattern, content)

with open("src/tasker/crm_open_sohail.rs", "w") as f:
    f.write(content)
