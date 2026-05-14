use http2::client::Builder;
use http2::frame::{PseudoId, PseudoOrder, SettingId, SettingsOrder, StreamDependency, StreamId};

use crate::profile::Http2Profile;
use crate::profile::PseudoOrder as QuikPseudoOrder;

/// Configures an HTTP/2 client builder with parameters that replicate a real Chrome 134 handshake.
///
/// Standard HTTP/2 libraries often use defaults that are trivial to detect (e.g., ascending
/// SETTINGS IDs or `:method :scheme :path :authority` pseudo-header order). This function
/// overrides those defaults to match Chromium's specialized transport behavior.
///
/// ## Key Enforcements
/// - **SETTINGS Order**: Explicitly set to [1, 2, 4, 6] (Header Table, Push, Window Size, Max Header List).
/// - **Absence of IDs 3/5**: Chrome does not send `MAX_CONCURRENT_STREAMS` or `MAX_FRAME_SIZE` in its initial settings.
/// - **Pseudo-header Sequence**: Reorders pseudo-headers to `m,a,s,p` (Method, Authority, Scheme, Path).
/// - **Priority Signaling**: Embeds a priority block in the initial HEADERS frame.
pub fn configure_builder(builder: &mut Builder, profile: &Http2Profile) {
    // 1. SETTINGS Frame Order [1, 2, 4, 6]
    // The sequence of these IDs is a high-entropy fingerprint signal used by WAFs.
    let mut settings_order = SettingsOrder::builder();
    settings_order = settings_order.push(SettingId::HeaderTableSize); // 1
    settings_order = settings_order.push(SettingId::EnablePush); // 2
    settings_order = settings_order.push(SettingId::InitialWindowSize); // 4
    settings_order = settings_order.push(SettingId::MaxHeaderListSize); // 6
    builder.settings_order(settings_order.build());

    // 2. SETTINGS Frame Values
    builder.header_table_size(profile.settings.header_table_size);
    builder.enable_push(profile.settings.enable_push);
    builder.initial_window_size(profile.settings.initial_window_size);
    builder.max_header_list_size(profile.settings.max_header_list_size);

    // 3. Connection-Level Window Update
    // Chrome immediately expands its connection window beyond the RFC default.
    builder.initial_connection_window_size(profile.initial_connection_window_size);

    // 4. Pseudo-header Sequence (m,a,s,p)
    // Moving `:authority` to the second position is the most recognizable Chrome H2 marker.
    let mut pseudo_order = PseudoOrder::builder();
    for id in &profile.pseudo_order {
        match id {
            QuikPseudoOrder::Method => {
                pseudo_order = pseudo_order.push(PseudoId::Method);
            }
            QuikPseudoOrder::Authority => {
                pseudo_order = pseudo_order.push(PseudoId::Authority);
            }
            QuikPseudoOrder::Scheme => {
                pseudo_order = pseudo_order.push(PseudoId::Scheme);
            }
            QuikPseudoOrder::Path => {
                pseudo_order = pseudo_order.push(PseudoId::Path);
            }
        }
    }
    builder.headers_pseudo_order(pseudo_order.build());

    // 5. HEADERS Priority Block
    // Chrome embeds priority metadata (dep=0, weight=256, exclusive=true) inside
    // the HEADERS frame rather than sending a separate PRIORITY frame.
    builder.headers_stream_dependency(StreamDependency::new(
        StreamId::ZERO,
        profile.headers_priority.weight,
        profile.headers_priority.exclusive,
    ));
}
