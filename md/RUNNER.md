# Runner Application GUI

The Runner GUI provides an interface to configure tasks, applications, and pipelines. It is accessible via HTTP according to `gui_host` and `gui_port` inside `runner_config.json`.

## Multiple Steps and Execution Modes
Tasks inside the Runner support executing multiple sequential or parallel actions within "Steps". You can configure an unlimited number of Steps and Actions inside the GUI by clicking **Add step** or **Add action**.

A single step has an execution mode:
- **Sequential**: Every action executes in order. The pipeline halts if an action fails.
- **Parallel**: Every action executes at the same time concurrently. The pipeline halts if any action fails.

Tasks can also configure **Post Run Steps**, which trigger their own isolated step pipeline if and only if the main step pipeline completes successfully.

When configuring an application's actions, the Runner GUI will dynamically inject the available parameters based on the executed application's manifest. This means Runner can seamlessly schedule Yasweb downloads, CRM fetching, or simple Shell Commands out of the box.

## API Endpoints
- `/run/{task_id}` (POST) - Forces immediate execution of the given task ID.
- `/run-all` (POST) - Enqueues all tasks for immediate execution.
