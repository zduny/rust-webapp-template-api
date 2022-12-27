use zzrpc::api;

#[api]
pub trait Api {
    /// Init and request user name.
    async fn user_name(&self) -> String;

    /// Get connected user names.
    async fn user_names(&self) -> Vec<String>;

    /// Send chat message.
    async fn message(&self, message: String);

    /// Stream of pairs containing: (user name, message)
    async fn messages(&self) -> impl Stream<Item = (String, String)>;

    /// Stream of names of newly connected users.
    async fn connected(&self) -> impl Stream<Item = String>;

    /// Stream of names of disconnected users.
    async fn disconnected(&self) -> impl Stream<Item = String>;
}