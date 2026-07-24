use soroban_sdk::contracterror;

/// Error variants shared across bridgelet contracts.
///
/// Both `ephemeral_account` and `sweep_controller` can encounter the same
/// failure modes (not initialized, already initialized, unauthorized, expired).
/// Centralising them here avoids duplicate numeric codes and ensures the SDK
/// can interpret errors from either contract with a single enum.
///
/// Contract-specific errors remain in their respective `errors.rs` files and
/// are re-exported from there.  Only errors that genuinely overlap belong
/// here.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum SharedError {
    /// Contract has not been initialized yet.
    NotInitialized = 1,
    /// Contract has already been initialized — double-init is forbidden.
    AlreadyInitialized = 2,
    /// Caller is not authorized to perform this action.
    Unauthorized = 3,
    /// The operation is only valid before the account has expired.
    Expired = 4,
    /// The operation is only valid after the account has expired.
    NotExpired = 5,
    /// An arithmetic overflow or underflow was detected.
    Overflow = 6,
    /// The provided amount is not positive (or is zero).
    InvalidAmount = 7,
}

// ── Conversions ────────────────────────────────────────────────────────
// Allow `?` propagation from shared errors into contract-specific error
// enums that define the same variants.  Each contract implements
// `From<SharedError>` so shared helper functions can return `SharedError`
// and the caller's `?` operator will auto-convert.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_error_codes_are_unique() {
        let codes = [
            SharedError::NotInitialized as u32,
            SharedError::AlreadyInitialized as u32,
            SharedError::Unauthorized as u32,
            SharedError::Expired as u32,
            SharedError::NotExpired as u32,
            SharedError::Overflow as u32,
            SharedError::InvalidAmount as u32,
        ];
        let mut sorted = codes;
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), codes.len(), "duplicate SharedError discriminant");
    }

    #[test]
    fn shared_error_codes_do_not_collide_with_ephemeral_account_errors() {
        // ephemeral_account::Error starts at 1 and goes up to 15.
        // SharedError variants must not reuse those codes when both
        // errors are present in the same transaction.
        let ephemeral_codes: std::vec::Vec<u32> = (1..=15).collect();
        let shared_codes = [
            SharedError::NotInitialized as u32,
            SharedError::AlreadyInitialized as u32,
            SharedError::Unauthorized as u32,
            SharedError::Expired as u32,
            SharedError::NotExpired as u32,
            SharedError::Overflow as u32,
            SharedError::InvalidAmount as u32,
        ];
        for code in &shared_codes {
            assert!(
                !ephemeral_codes.contains(code),
                "SharedError code {} collides with ephemeral_account::Error",
                code
            );
        }
    }

    #[test]
    fn shared_error_codes_do_not_collide_with_sweep_controller_errors() {
        // sweep_controller::Error has codes 1–13 (with gap at 12).
        let sweep_codes: std::vec::Vec<u32> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13];
        let shared_codes = [
            SharedError::NotInitialized as u32,
            SharedError::AlreadyInitialized as u32,
            SharedError::Unauthorized as u32,
            SharedError::Expired as u32,
            SharedError::NotExpired as u32,
            SharedError::Overflow as u32,
            SharedError::InvalidAmount as u32,
        ];
        for code in &shared_codes {
            assert!(
                !sweep_codes.contains(code),
                "SharedError code {} collides with sweep_controller::Error",
                code
            );
        }
    }
}
