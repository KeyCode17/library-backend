//! RBAC: roles and the permissions they grant.

use serde::{Deserialize, Serialize};

/// A user role. Serializes as a lowercase string (`"admin"`, ...) to match the
/// `Role` enum in the API contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Librarian,
    Member,
}

/// A capability a role may grant. Endpoints are protected by required permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    /// Assign roles to other users (admin-only).
    ManageUsers,
    /// Create/update catalog entries (future write slices).
    ManageCatalog,
    /// Approve/return loans (future lending slice).
    ManageLoans,
    /// Borrow books (future lending slice).
    BorrowBooks,
}

impl Role {
    /// The permissions this role grants.
    pub fn permissions(self) -> &'static [Permission] {
        match self {
            Role::Admin => &[
                Permission::ManageUsers,
                Permission::ManageCatalog,
                Permission::ManageLoans,
                Permission::BorrowBooks,
            ],
            Role::Librarian => &[
                Permission::ManageCatalog,
                Permission::ManageLoans,
                Permission::BorrowBooks,
            ],
            Role::Member => &[Permission::BorrowBooks],
        }
    }

    /// Whether this role grants `permission`.
    pub fn grants(self, permission: Permission) -> bool {
        self.permissions().contains(&permission)
    }

    /// Stable wire string for this role.
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Librarian => "librarian",
            Role::Member => "member",
        }
    }

    /// Parse a role from its wire string.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "admin" => Some(Role::Admin),
            "librarian" => Some(Role::Librarian),
            "member" => Some(Role::Member),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_admin_can_manage_users() {
        assert!(Role::Admin.grants(Permission::ManageUsers));
        assert!(!Role::Librarian.grants(Permission::ManageUsers));
        assert!(!Role::Member.grants(Permission::ManageUsers));
    }

    #[test]
    fn wire_string_roundtrips() {
        for role in [Role::Admin, Role::Librarian, Role::Member] {
            assert_eq!(Role::parse(role.as_str()), Some(role));
        }
        assert_eq!(Role::parse("root"), None);
    }
}
