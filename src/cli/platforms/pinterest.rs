use crate::api::AppState;
use crate::cli::PinterestAction;

pub async fn handle(action: PinterestAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        PinterestAction::Profile { board_id } => {
            let input = crate::mcp::tools_pinterest::PiGetUserAccountInput { board_id };
            crate::mcp::tools_pinterest::handle_pi_get_user_account(state, &input).await.map(|v| v.0)
        }
        PinterestAction::Board { board_id } => {
            let input = crate::mcp::tools_pinterest::PiGetBoardInput { board_id };
            crate::mcp::tools_pinterest::handle_pi_get_board(state, &input).await.map(|v| v.0)
        }
        PinterestAction::Pins { board_id, limit } => {
            let input = crate::mcp::tools_pinterest::PiGetBoardPinsInput { board_id, limit: Some(limit) };
            crate::mcp::tools_pinterest::handle_pi_get_board_pins(state, &input).await.map(|v| v.0)
        }
        PinterestAction::Pin { board_id, pin_id } => {
            let input = crate::mcp::tools_pinterest::PiGetPinInput { board_id, pin_id };
            crate::mcp::tools_pinterest::handle_pi_get_pin(state, &input).await.map(|v| v.0)
        }
        PinterestAction::Search { query, limit } => {
            let input = crate::mcp::tools_pinterest::PiSearchPinsInput { query, limit: Some(limit) };
            crate::mcp::tools_pinterest::handle_pi_search_pins(state, &input).await.map(|v| v.0)
        }
        PinterestAction::BoardAnalytics { board_id, start_date, end_date } => {
            let input = crate::mcp::tools_pinterest::PiGetBoardAnalyticsInput { board_id, start_date, end_date };
            crate::mcp::tools_pinterest::handle_pi_get_board_analytics(state, &input).await.map(|v| v.0)
        }
        PinterestAction::PinAnalytics { board_id, pin_id, start_date, end_date } => {
            let input = crate::mcp::tools_pinterest::PiGetPinAnalyticsInput { board_id, pin_id, start_date, end_date };
            crate::mcp::tools_pinterest::handle_pi_get_pin_analytics(state, &input).await.map(|v| v.0)
        }
    };

    super::emit_result(result)
}
