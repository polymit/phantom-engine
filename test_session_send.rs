use phantom_js::tier1::session::Tier1Session;

fn require_send<T: Send>() {}

fn main() {
    require_send::<Tier1Session>();
}
