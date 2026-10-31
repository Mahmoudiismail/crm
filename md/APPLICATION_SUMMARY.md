# Application Summary

The CRM Tool repository is one Rust package with five executable targets: runner, crm, yasweb, tasker, and crm_updater.

## Capabilities

- Schedule and orchestrate local applications through Runner's GUI, scheduler, execution queue and multi-step pipelines.
- Authenticate to the CRM service and download ticket, call, lead and user exports.
- Automate configured Yasweb/MIS reports through headless Chrome.
- Enrich and transform CSV reports, refresh Excel dashboards, split workbooks, and create Outlook messages through Tasker.
- Collect local logs/reports and apply staged executable updates through CRM Updater.

## Binary responsibilities

| Binary | Primary role | Configuration |
|---|---|---|
| runner | Scheduler, local GUI, system tray and task pipelines | runner_config.json |
| crm | CRM API fetcher and report downloader | config.json |
| yasweb | Browser-based report extraction | yasweb_config.json |
| tasker | CSV, Excel and email task processing | tasker_config.json |
| crm_updater | Local collection and executable replacement | updater_config.json |

## Runner execution model

Runner evaluates configured schedules and dispatches task pipelines. A pipeline contains ordered steps; each step runs its actions sequentially or in parallel. Post-run steps execute only after the main pipeline succeeds. The execution manager coordinates queued/running task admission, and application locks prevent overlapping use of non-concurrent applications.

External applications can expose an AppManifest so Runner can generate argument forms from the application's declared CLI contract. Date controls and date argument injection are conditional on supported manifest arguments.

## Tasker lead result

For CSV Analysis, Tasker writes the ticket result to the configured output_file. If recent lead report exports are available, it separately writes lead_result.csv beside that ticket result. The lead's team is looked up from Assignee in the configured users mapping. Missing or unknown assignees remain unassigned. The Call Center attachment is filtered to Call Center team leads with status New or Follow-up and excludes configured branches; other teams remain in lead_result.csv.

## Documentation map

Start at [md/README.md](./README.md). See [Architecture](./ARCHITECTURE.md), [Configuration](./CONFIG.md), [CLI](./CLI.md), [Runner](./RUNNER.md), [Tasker](./TASKER.md), [CRM Updater](./CRM_UPDATER.md), and [Operations](./OPERATIONS.md).
