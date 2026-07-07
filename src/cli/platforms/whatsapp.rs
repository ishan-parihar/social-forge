use crate::api::AppState;
use crate::cli::WhatsappAction;

pub async fn handle(action: WhatsappAction, state: &AppState) -> anyhow::Result<()> {
    let result: Result<serde_json::Value, String> = match action {
        WhatsappAction::Send { to, text } => {
            let input = crate::mcp::tools_whatsapp::WaSendTextInput { to, text };
            crate::mcp::tools_whatsapp::handle_wa_send_text(state, &input)
                .await
                .map(|v| serde_json::to_value(v.0).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        WhatsappAction::Chats { limit } => {
            let input = crate::mcp::tools_whatsapp::WaChatsInput { limit: Some(limit) };
            crate::mcp::tools_whatsapp::handle_wa_chats(state, &input)
                .await
                .map(|v| serde_json::to_value(v.0).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        WhatsappAction::Contacts { query, limit } => {
            let input = crate::mcp::tools_whatsapp::WaContactsInput { query, limit: Some(limit) };
            crate::mcp::tools_whatsapp::handle_wa_contacts(state, &input)
                .await
                .map(|v| serde_json::to_value(v.0).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        WhatsappAction::Groups => {
            crate::mcp::tools_whatsapp::handle_wa_list_groups(state)
                .await
                .map(|v| serde_json::to_value(v.0).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        WhatsappAction::CreateGroup { name, participants } => {
            let input = crate::mcp::tools_whatsapp::WaCreateGroupInput { subject: name, participants };
            crate::mcp::tools_whatsapp::handle_wa_create_group(state, &input)
                .await
                .map(|v| serde_json::to_value(v.0).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
        WhatsappAction::InviteLink { group_jid } => {
            let input = crate::mcp::tools_whatsapp::WaGroupInviteLinkInput { group_jid };
            crate::mcp::tools_whatsapp::handle_wa_group_invite_link(state, &input)
                .await
                .map(|v| serde_json::to_value(v.0).unwrap_or_default())
                .map_err(|e| e.to_string())
        }
    };

    super::emit_result(result)
}
