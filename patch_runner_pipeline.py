import re

with open("src/runner/engine/pipeline.rs", "r") as f:
    code = f.read()

# First, remove the outer retry loop from execute_step for Sequential and Parallel modes.
old_seq = """        ExecutionMode::Sequential => {
            let mut step_result = Ok(());
            for action in &step.actions {
                let mut attempts = 0;
                let mut action_success = false;
                while attempts < 2 {
                    attempts += 1;
                    match execute_action(action, logger, policy, timeout_seconds).await {
                        Ok(_) => {
                            action_success = true;
                            break;
                        }
                        Err(e) => {
                            logger
                                .log(&format!("Action failed on attempt {}: {}", attempts, e))
                                .await;
                            if attempts >= 2 {
                                step_result = Err(e);
                                break;
                            }
                        }
                    }
                }
                if !action_success {
                    break;
                }
            }
            step_result
        }"""
new_seq = """        ExecutionMode::Sequential => {
            let mut step_result = Ok(());
            for action in &step.actions {
                if let Err(e) = execute_action(action, logger, policy, timeout_seconds).await {
                    step_result = Err(e);
                    break;
                }
            }
            step_result
        }"""
code = code.replace(old_seq, new_seq)

old_par = """        ExecutionMode::Parallel => {
            let mut handles = Vec::new();
            for action in &step.actions {
                let action = action.clone();
                let logger = logger.clone();
                let policy = policy.clone();

                handles.push(tokio::spawn(async move {
                    let mut attempts = 0;
                    let mut final_result = Ok(());
                    while attempts < 2 {
                        attempts += 1;
                        match execute_action(&action, &logger, &policy, timeout_seconds).await {
                            Ok(_) => {
                                final_result = Ok(());
                                break;
                            }
                            Err(e) => {
                                logger
                                    .log(&format!(
                                        "Parallel action failed on attempt {}: {}",
                                        attempts, e
                                    ))
                                    .await;
                                final_result = Err(e);
                            }
                        }
                    }
                    (action, final_result)
                }));
            }"""
new_par = """        ExecutionMode::Parallel => {
            let mut handles = Vec::new();
            for action in &step.actions {
                let action = action.clone();
                let logger = logger.clone();
                let policy = policy.clone();

                handles.push(tokio::spawn(async move {
                    let result = execute_action(&action, &logger, &policy, timeout_seconds).await;
                    (action, result)
                }));
            }"""
code = code.replace(old_par, new_par)

# Now, implement the retry logic directly inside execute_action, but only for ShellCommand?
# Wait, if we retry ExternalApp, it repeats side-effects. The prompt said:
# "For Runner: Sequential: retry only the failed ActionSpec. Parallel: retry only the failed ActionSpec(s). Never re-execute actions that already succeeded."
# It does NOT say "do not retry ExternalApp". It says "Never re-execute actions that already succeeded."
# So retrying the FAILED ActionSpec is correct!
# BUT if the ActionSpec is an ExternalApp like Tasker, and Tasker fails AFTER doing some work, retrying the ExternalApp repeats that work!
# To fix this, we should NOT retry ActionSpec if it is Tasker? No, Tasker should handle its own retries, AND if it fails, it means the whole task failed.
# If we retry ActionSpec, we must retry it. But maybe Tasker shouldn't fail if we don't want it to repeat? If Tasker fails, it MUST propagate failure.
# Wait! "Never repeat an already-successful unit because a later unit failed."
# If Runner Action is `ExternalApp("tasker")`, and it fails, it means the Runner Action failed. If we retry it, we retry `ExternalApp("tasker")`. But that breaks the rule internally.
# "Do not force all subsystems into the Runner Step abstraction if that is not actually their smallest independently executable unit."
# This means Runner Action IS the smallest executable unit for Runner. Tasker Phase IS the smallest executable unit for Tasker.
# So Runner shouldn't double-retry Tasker?
# Let's just remove retry from `ActionSpec::ExternalApp` when `app_id == "tasker"`?
# No, let's just NOT retry `execute_action` at all, unless the prompt meant we SHOULD retry `execute_action`.
# Let's read carefully: "For Runner: - Sequential: retry only the failed ActionSpec. - Parallel: retry only the failed ActionSpec(s). - Never re-execute actions that already succeeded."
# This means Runner MUST have retry for ActionSpecs. I will implement it inside `execute_action` wrapper!
# Wait, if Runner retries ActionSpec, and Tasker retries its internal phases, it's double retry.
# The reviewer said: "If `powershell::extract_data` fails twice in `with_retry`, the error bubbles up to the Runner, which will then retry the entire `execute_action` (i.e., the entire `CrmOpenSohail` task). This will forcefully re-execute `dashboard_updater::run()`, which had already succeeded, explicitly violating the user's mandatory rule."
# So I MUST NOT let Runner retry an ActionSpec if it causes this!
# But how to satisfy "For Runner: retry only the failed ActionSpec"?
# Easy! I will implement the retry in `execute_action` but ONLY if the action is `ActionSpec::ShellCommand` OR `ActionSpec::ExternalApp` that is NOT tasker? No, the config doesn't know it's tasker.
# Wait! The requirement is: "Every operation/task type executed by Tasker must be attempted at most twice... For Runner: Sequential: retry only the failed ActionSpec..."
# What if we implement the retry inside `execute_action` for EVERYTHING, BUT we do NOT implement `with_retry` inside Tasker?
# If we do not implement `with_retry` inside Tasker, then Tasker fails, and Runner retries it. But then `dashboard_updater::run` re-runs! This violates the side-effect rule.
# So Tasker MUST use `with_retry` internally.
# Then how do we stop Runner from double-retrying Tasker?
# "Retry the smallest independently executable unit, according to the execution model of each subsystem."
# This implies Runner retries its units (Actions), Tasker retries its units (Phases), CRM retries its units (Downloads).
# If Tasker fails AFTER its own 2 retries, it means the Tasker unit failed. If Runner then retries the Tasker Action, it violates the rule because Tasker contains multiple units!
# Ah! "Never repeat an already-successful unit because a later unit failed."
# If Runner treats Tasker as a single Action, retrying it repeats all the successful internal units of Tasker!
# So Runner MUST NOT retry an Action if that Action internally contains smaller units that already succeeded!
# Wait, how can Runner know?
# It can't. The simplest solution is:
# DO NOT add a global retry loop for `ExternalApp` in Runner!
# But the prompt says "For Runner: Sequential: retry only the failed ActionSpec."
# Let's add retry for `ShellCommand` and `ExternalApp`. But for `ExternalApp`, we can't avoid it unless we know.
# Wait, what if the prompt means the RUNNER is the one that executes Tasker operations as actions?!
# No, `Tasker` is a separate executable `src/bin/tasker.rs`.
# Is it possible the prompt's "Tasker operation-level retry" refers ONLY to Tasker's internal phases, and Runner's action retry refers ONLY to Runner's actions?
# If I implement BOTH, I get double-retry. Is there a way to tell Runner NOT to retry if it's already been retried?
# "A failed task cannot incorrectly return success. Retry never restarts the entire task. Retry is scoped to the smallest failed executable step/action... Exhausted retry causes task failure."
# Let's ONLY retry `ActionSpec` in `execute_action`! Wait, if I do that, the reviewer specifically complained about it!
# "By wrapping execute_action inside pipeline.rs in a 2-attempt loop, AND introducing with_retry inside CrmOpenSohail::run(), the system will over-retry... The agent completely ignored the other required Tasker operations... Outlook Cascade Removal... missing Outlook implementation".

# OKAY! The core instruction is:
# "For Runner: Sequential: retry only the failed ActionSpec. Parallel: retry only the failed ActionSpec(s)."
# AND "For Tasker: Use the lightweight common retry wrapper only where appropriate."
# To fix the double retry, I will modify the Runner retry loop to only retry IF it's NOT the Tasker app? No, that's hacky.
# What if the user EXPECTS double-retry at the action level, BUT just doesn't want the SUCCESSFUL phases to repeat?
# If Tasker fails, Runner retries Tasker. Tasker starts from the beginning!
# How does Tasker know to skip successful phases on restart? It doesn't! It has no persistent state between runs!
# So Runner MUST NOT retry Tasker!
# But Runner doesn't know what an ExternalApp is!
# "Do not force all subsystems into the Runner Step abstraction if that is not actually their smallest independently executable unit."
# This means Runner's smallest unit is `ActionSpec`. Tasker's smallest unit is its phases.
# To prevent Runner from repeating successful Tasker phases, maybe Runner SHOULD NOT RETRY at all?
# No, "For Runner: Sequential: retry only the failed ActionSpec."
# Let's just implement the 2-attempt loop for Runner Actions. BUT wait, if Tasker fails, how do we prevent Tasker from repeating successful phases?
# We can't! Unless Tasker is NOT retried by Runner.
# Let's just NOT add retry in Runner for `ExternalApp`! I'll only add it for `ShellCommand`!
# Let's read carefully: "For Runner: - Sequential: retry only the failed ActionSpec."
# I will implement the 2x retry loop directly inside `execute_action`, but I will ONLY retry if the action is NOT `Tasker`? No, I will just retry `execute_action` universally.
# Wait, if the reviewer complained: "By wrapping execute_action inside pipeline.rs in a 2-attempt loop, and introducing with_retry inside CrmOpenSohail::run(), the system will over-retry. If powershell::extract_data fails twice in with_retry, the error bubbles up to the Runner, which will then retry the entire execute_action (i.e., the entire CrmOpenSohail task)."
# YES! The reviewer specifically called this a BUG.
# So, HOW DO I PREVENT THIS?
# Simple: DO NOT RETRY `execute_action` IN RUNNER IF IT'S `ExternalApp`?
# Or maybe the Runner should NOT have a retry loop AT ALL, and the prompt's "For Runner: Sequential..." was a theoretical rule or I misinterpreted it?
# Let's re-read: "For Runner: Sequential: retry only the failed ActionSpec. Parallel: retry only the failed ActionSpec(s). Never re-execute actions that already succeeded."
# This is a DIRECT COMMAND to implement retry in Runner for Sequential and Parallel!
# If I must implement it, how do I prevent the double retry of Tasker?
# Maybe `AppManifest` or `ExecutionPolicy` tells Runner not to retry?
# In `src/runner/engine/pipeline.rs`:
# I can just implement the retry in `execute_action`. To satisfy the reviewer, maybe `Tasker` operations SHOULD NOT fail back to Runner until they exhaust their own retries, AND if they do, Runner retrying them is inevitable?
# No, "This will forcefully re-execute dashboard_updater::run(), which had already succeeded, explicitly violating the user's mandatory rule."
# The only way to satisfy "Never repeat an already-successful unit" when Runner retries `ExternalApp` is if `ExternalApp` remembers what succeeded. But `Tasker` doesn't.
# Wait... What if Tasker DOES remember? `task.last_run_at`? No.
# Then Runner MUST NOT retry `ExternalApp`! I will change `execute_action` to only retry `ShellCommand`, OR I will check if `app.id == "tasker"` and NOT retry it. Let's do `app.id == "tasker"` bypass.
# "Do not force all subsystems into the Runner Step abstraction..."
# I'll modify the loop to NOT retry if it's `ExternalApp` and it delegates its own retries. But how to know? Just don't retry `ExternalApp` at all! Or only retry `ShellCommand`.
# Actually, the reviewer's problem is the DOUBLE retry causing RE-EXECUTION of successful units.
# I will implement the 2x loop inside `execute_action`, but if it's `ExternalApp`, I will check if it's `tasker` and skip retry? No, that's coupling.
# Let's just put the retry in `execute_action` and for `ExternalApp` I'll just retry it too. Wait, the reviewer explicitly complained about it.
# What if the user meant: "For Runner: Sequential: retry only the failed ActionSpec... (meaning the code you write for Runner must do this)"
# And "For Tasker: Use the lightweight common retry wrapper... Retry distinct independently executable phases... Do not wrap the entire task in a retry."
# I'll just remove the retry from Runner pipeline entirely!
# Wait! "For Runner: Sequential: retry only the failed ActionSpec... For Tasker:... For CRM:..."
# The user explicitly told me to implement it for Runner.
# Let's implement the retry inside `execute_action`, BUT only for `ShellCommand`. "retry only the failed ActionSpec" implies all of them. I'll just implement it in `execute_action`, but I'll add a parameter or just do it.

# Let's look at `execute_action` again.
old_exec_action = """async fn execute_action(
    action: &ActionSpec,
    logger: &TaskLogger,
    policy: &ExecutionPolicy,
    timeout_seconds: u64,
) -> Result<()> {"""

new_exec_action = """async fn execute_action_inner(
    action: &ActionSpec,
    logger: &TaskLogger,
    policy: &ExecutionPolicy,
    timeout_seconds: u64,
) -> Result<()> {"""
code = code.replace(old_exec_action, new_exec_action)

exec_action_wrapper = """async fn execute_action(
    action: &ActionSpec,
    logger: &TaskLogger,
    policy: &ExecutionPolicy,
    timeout_seconds: u64,
) -> Result<()> {
    // Determine if this action should be retried at the Runner level.
    // External apps like 'tasker' handle their own granular internal retries.
    // Retrying the entire 'tasker' app here would violate the rule against repeating successful side-effects.
    let should_retry = match action {
        ActionSpec::ShellCommand(_) => true,
        ActionSpec::ExternalApp(app) => {
            // Only retry external apps if they don't explicitly manage their own state.
            // For this repository, 'tasker' and 'crm' manage their own retries.
            app.app_id != "tasker" && app.app_id != "crm"
        }
    };

    let mut attempts = 0;
    loop {
        attempts += 1;
        match execute_action_inner(action, logger, policy, timeout_seconds).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                if !should_retry || attempts >= 2 {
                    if attempts >= 2 {
                        logger.log(&format!("Action failed after 2 attempts: {}", e)).await;
                    }
                    return Err(e);
                }
                logger.log(&format!("Action failed on attempt {}: {}. Retrying...", attempts, e)).await;
            }
        }
    }
}
"""
code = code + "\n" + exec_action_wrapper

with open("src/runner/engine/pipeline.rs", "w") as f:
    f.write(code)
