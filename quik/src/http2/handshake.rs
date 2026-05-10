use http2::client::Builder;
use http2::frame::{PseudoId, PseudoOrder, SettingId, SettingsOrder, StreamDependency, StreamId};

use crate::profile::Http2Profile;
use crate::profile::PseudoOrder as QuikPseudoOrder;

/// Configures the HTTP/2 builder with Chrome 134-identical handshake parameters.
///
/// This enforces:
/// - SETTINGS order [1, 2, 4, 6]
/// - Chrome's specific SETTINGS values
/// - Pseudo-header ordering (:method, :authority, :scheme, :path)
/// - Initial connection window size with Chrome delta
/// - Priority block in HEADERS
pub fn configure_builder(builder: &mut Builder, profile: &Http2Profile) {
    // 1. SETTINGS Order [1, 2, 4, 6]
    let mut settings_order = SettingsOrder::builder();
    settings_order = settings_order.push(SettingId::HeaderTableSize); // 1
    settings_order = settings_order.push(SettingId::EnablePush); // 2
    settings_order = settings_order.push(SettingId::InitialWindowSize); // 4
    settings_order = settings_order.push(SettingId::MaxHeaderListSize); // 6
    builder.settings_order(settings_order.build());

    // 2. SETTINGS Values
    builder.header_table_size(profile.settings.header_table_size);
    builder.enable_push(profile.settings.enable_push);
    builder.initial_window_size(profile.settings.initial_window_size);
    builder.max_header_list_size(profile.settings.max_header_list_size);

    // 3. Connection Window
    builder.initial_connection_window_size(profile.initial_connection_window_size);

    // 4. Pseudo-header Order
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

    // 5. Stream Priority (HEADERS priority block)
    // Chrome uses dep=0, weight=255, exclusive=true for the initial request.
    builder.headers_stream_dependency(StreamDependency::new(
        StreamId::ZERO, // dep=0
        profile.headers_priority.weight,
        profile.headers_priority.exclusive,
    ));
}
