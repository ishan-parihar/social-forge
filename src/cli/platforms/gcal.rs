use crate::api::AppState;
use crate::cli::GcalAction;

pub async fn handle(action: GcalAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        GcalAction::Calendars => {
            let input = crate::mcp::tools_google::GcalListCalendarsInput { max_results: None };
            crate::mcp::tools_google::handle_goog_list_calendars(state, &input).await.map(|v| v.0)
        }
        GcalAction::Events { calendar_id, limit } => {
            let input = crate::mcp::tools_google::GcalListEventsInput {
                calendar_id, max_results: Some(limit), time_min: None, time_max: None,
            };
            crate::mcp::tools_google::handle_goog_list_events(state, &input).await.map(|v| v.0)
        }
        GcalAction::Event { calendar_id, event_id } => {
            let input = crate::mcp::tools_google::GcalGetEventInput { calendar_id, event_id };
            crate::mcp::tools_google::handle_goog_get_event(state, &input).await.map(|v| v.0)
        }
        GcalAction::Create { calendar_id, title, start, end, description } => {
            let input = crate::mcp::tools_google::GcalCreateEventInput {
                calendar_id, summary: title, description, start_time: start, end_time: end, timezone: None,
            };
            crate::mcp::tools_google::handle_goog_create_event(state, &input).await.map(|v| v.0)
        }
        GcalAction::Update { calendar_id, event_id, title, description, start, end } => {
            let input = crate::mcp::tools_google::GcalUpdateEventInput {
                calendar_id,
                event_id,
                summary: title,
                description,
                start_time: start,
                end_time: end,
            };
            crate::mcp::tools_google::handle_goog_update_event(state, &input).await.map(|v| v.0)
        }
        GcalAction::Delete { calendar_id, event_id } => {
            let input = crate::mcp::tools_google::GcalDeleteEventInput { calendar_id, event_id };
            crate::mcp::tools_google::handle_goog_delete_event(state, &input).await.map(|v| v.0)
        }
    };
    match result {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
        Err(e) => { eprintln!("{}", serde_json::json!({"error": e})); std::process::exit(1); }
    }
    Ok(())
}
