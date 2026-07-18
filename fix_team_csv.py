import re

with open("src/tasker/crm_open_sohail.rs", "r") as f:
    content = f.read()

search_pattern = r"""        } else if \(h_lower == "owner"
            \|\| h_lower == "receiver name"
            \|\| h_lower == "oul"
            \|\| h_lower\.contains\("owner"\)
            \|\| h_lower\.contains\("receiver"\)\)
            && owner_idx\.is_none\(\)
        \{
            owner_idx = Some\(i\);
        \} else if \(h_lower == "to emails"
            \|\| h_lower == "email"
            \|\| h_lower == "email_to"
            \|\| h_lower\.contains\("email"\)\)
            && email_idx\.is_none\(\)
        \{
            email_idx = Some\(i\);
        \}"""

replace_pattern = r"""        } else if (h_lower == "owner_name"
            || h_lower == "owner"
            || h_lower == "receiver name"
            || h_lower == "oul"
            || h_lower.contains("owner")
            || h_lower.contains("receiver"))
            && owner_idx.is_none()
        {
            owner_idx = Some(i);
        } else if (h_lower == "owner_email"
            || h_lower == "to emails"
            || h_lower == "email"
            || h_lower == "email_to"
            || h_lower.contains("email"))
            && email_idx.is_none()
        {
            email_idx = Some(i);
        }"""

content = re.sub(search_pattern, replace_pattern, content)

with open("src/tasker/crm_open_sohail.rs", "w") as f:
    f.write(content)
