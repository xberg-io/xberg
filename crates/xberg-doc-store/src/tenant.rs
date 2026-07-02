//! Tenant/actor identity threaded through every sidecar-store call.

/// Newtype over a tenant identifier. Opaque to this crate — a store instance
/// (SQLite table, pgvector schema, …) partitions all queries by this value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TenantId(pub String);

/// Newtype over an actor identifier, used for audit attribution.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActorId(pub String);

/// The tenant/actor pair every sidecar-store call is scoped to.
///
/// Carried in every trait method signature from day one (per the design
/// spec's resolved tenancy decision) even when the caller only ever
/// constructs [`TenantCtx::default_tenant`] — so enabling multi-tenancy later
/// is a backend swap, not a breaking API change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantCtx {
    /// The trust domain this call is scoped to.
    pub tenant: TenantId,
    /// The identity performing the call (for audit attribution).
    pub actor: ActorId,
}

impl TenantCtx {
    /// Construct an explicit tenant/actor context.
    pub fn new(tenant: impl Into<String>, actor: impl Into<String>) -> Self {
        Self {
            tenant: TenantId(tenant.into()),
            actor: ActorId(actor.into()),
        }
    }

    /// The single-tenant context used until a `TenantResolver` (auth layer,
    /// out of scope for this crate) is wired into the API.
    pub fn default_tenant() -> Self {
        Self::new("default", "anonymous")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tenant_uses_default_tenant_id() {
        let ctx = TenantCtx::default_tenant();
        assert_eq!(ctx.tenant, TenantId("default".to_string()));
        assert_eq!(ctx.actor, ActorId("anonymous".to_string()));
    }

    #[test]
    fn new_wraps_supplied_values() {
        let ctx = TenantCtx::new("acme", "user-42");
        assert_eq!(ctx.tenant.0, "acme");
        assert_eq!(ctx.actor.0, "user-42");
    }
}
