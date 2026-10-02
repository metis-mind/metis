//! Runtime glue and the host router (ADR-0011): actor worker loops over
//! bounded mailboxes, the four event dispatch modes (ADR-0014), the service
//! registry with epoch reverse index (ADR-0018), and journal capture.
//! Everything asynchronous lives here, on the impure side of the
//! `metis-fiber-core` boundary (ADR-0019).
