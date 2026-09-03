import re
with open("src/tasker/crm_open_sohail/mod.rs", "r") as f:
    c = f.read()

# The original `test_outlook_reply_all_draft_mechanism` has some hardcoded strings
# Let's remove our custom `test_task3_powershell_generation_rules` and just update `test_outlook_reply_all_draft_mechanism`.

# Let's see what failed:
# 'tasker::crm_open_sohail::tests::test_outlook_reply_all_draft_mechanism' panicked at 'Should never call Send() on the generated email'
# That's because the word "Send" might be in the code somewhere! Wait, let's look at `test_outlook_reply_all_draft_mechanism`
#         let bad_send = "$ReplyMail.Se";
#         let bad_send2 = "nd()";
# Oh wait, $Item.SenderEmailAddress ... $SenderAddress
# Wait, "Se" + "nd()" is "Send()", maybe I used "Send" somewhere? Let's check.

# For `test_task3_powershell_generation_rules`:
# `Should never assign To`. Wait, I did `assert!(!src.contains("$ReplyMail.To ="))`. Is there `$ReplyMail.To` somewhere? No, I deleted it from the code.
# But wait, `src.contains` checks the whole `mod.rs` file! And the `mod.rs` file contains the test itself!
# The test contains the string `"$ReplyMail.To ="`, so `src.contains("$ReplyMail.To =")` is TRUE because the test string literal is inside `src`!

c = c.replace('!src.contains("$ReplyMail.To =")', '!src.contains(&format!("$ReplyMail.{} = ", "To"))')
c = c.replace('!src.contains("$ReplyMail.CC =")', '!src.contains(&format!("$ReplyMail.{} = ", "CC"))')
c = c.replace('!src.contains("$ReplyMail.Send()")', '!src.contains(&format!("$ReplyMail.{}()", "Send"))')

# Let's remove `test_task3_powershell_generation_rules` and combine logic in `test_outlook_reply_all_draft_mechanism`.
c = re.sub(r'#\[test\]\s*fn test_task3_powershell_generation_rules\(\) \{[\s\S]*?\}\n', '', c)

c = c.replace('assert!(\n            !src.contains(&format!("{bad_send}{bad_send2}")),\n            "Should never call Send() on the generated email"\n        );', 'assert!(!src.contains(&format!("$ReplyMail.{}()", "Send")), "Should never call Send() on the generated email");')
c = c.replace('let bad_send = "$ReplyMail.Se";\n        let bad_send2 = "nd()";', '')
c = c.replace('assert!(!src.contains(&format!("{bad_send}{bad_send2}")), "Should never call Send() on the generated email");', 'assert!(!src.contains(&format!("$ReplyMail.{}()", "Send")), "Should never call Send() on the generated email");')

c = c.replace('''    #[test]
    fn test_outlook_reply_all_draft_mechanism() {''', '''    #[test]
    fn test_outlook_reply_all_draft_mechanism() {
        let src = include_str!("mod.rs");
        assert!(src.contains("GetExchangeUser()"));
        assert!(src.contains("PrimarySmtpAddress"));
        assert!(src.contains("0x39FE001E"));
        assert!(src.contains("catch"));
        assert!(!src.contains(&format!("$ReplyMail.{} = ", "To")));
        assert!(!src.contains(&format!("$ReplyMail.{} = ", "CC")));''')

with open("src/tasker/crm_open_sohail/mod.rs", "w") as f:
    f.write(c)
