//! # Tokio Cancellation Extensions
//!
//! This crate provides extension traits for adding cancellation support to Tokio futures
//! in a clean and ergonomic way, eliminating the need for repetitive `tokio::select!` boilerplate.
//!
//! ## Problem
//!
//! When working with async Rust and Tokio, adding cancellation support to futures often
//! requires repetitive `tokio::select!` boilerplate:
//!
//! ```ignore
//! // Repetitive boilerplate code
//! let result = tokio::select! {
//!     _ = cancellation_token.cancelled() => {
//!         return Err(MyError::Cancelled);
//!     }
//!     result = some_future => {
//!         result?
//!     }
//! };
//! ```
//!
//! ## Solution
//!
//! This crate provides a simple extension trait that eliminates this boilerplate:
//!
//! ```rust
//! use tokio_cancellation_ext::{CancellationExt, CancellationError};
//! use tokio_util::sync::CancellationToken;
//!
//! #[derive(Debug)]
//! enum MyError {
//!     Cancelled,
//!     Database(String),
//! }
//!
//! impl From<CancellationError> for MyError {
//!     fn from(_: CancellationError) -> Self {
//!         MyError::Cancelled
//!     }
//! }
//!
//! # async fn fetch_data() -> Result<String, MyError> { Ok("data".to_string()) }
//! # #[tokio::main]
//! # async fn main() -> Result<(), MyError> {
//! let token = CancellationToken::new();
//!
//! // Clean, readable code without boilerplate
//! let result = fetch_data()
//!     .with_cancellation(&token, "fetch_data")
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Features
//!
//! - **Zero boilerplate**: Replace `tokio::select!` with simple `.with_cancellation()` calls
//! - **Type-safe**: Works with any error type that can convert from `CancellationError`
//! - **Thread-safe**: Fully `Send + Sync` compatible
//! - **Ergonomic**: Chainable API that feels natural
//! - **Logging support**: Built-in context logging for debugging
//! - **Minimal dependencies**: Only requires `tokio-util` and `log`
//!
//! ## Usage Examples
//!
//! ### Basic Usage
//!
//! ```rust
//! use tokio_cancellation_ext::{CancellationExt, CancellationError};
//! use tokio_util::sync::CancellationToken;
//!
//! #[derive(Debug)]
//! enum AppError {
//!     Cancelled,
//!     Network(String),
//!     Database(String),
//! }
//!
//! impl From<CancellationError> for AppError {
//!     fn from(_: CancellationError) -> Self {
//!         AppError::Cancelled
//!     }
//! }
//!
//! # async fn network_call() -> Result<String, AppError> { Ok("response".to_string()) }
//! # async fn database_query() -> Result<Vec<u8>, AppError> { Ok(vec![1, 2, 3]) }
//! # #[tokio::main]
//! # async fn main() -> Result<(), AppError> {
//! let token = CancellationToken::new();
//!
//! // Multiple cancellable operations
//! let response = network_call()
//!     .with_cancellation(&token, "network_call")
//!     .await?;
//!
//! let data = database_query()
//!     .with_cancellation(&token, "database_query")
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### With Error Conversion
//!
//! The trait works with any error type that implements `From<CancellationError>`:
//!
//! ```rust
//! use tokio_cancellation_ext::{CancellationExt, CancellationError};
//! use tokio_util::sync::CancellationToken;
//!
//! // Works with anyhow
//! # #[cfg(feature = "anyhow")]
//! # {
//! use anyhow::Result;
//!
//! async fn example_with_anyhow(token: &CancellationToken) -> Result<String> {
//!     let result = some_async_operation()
//!         .with_cancellation(token, "async_operation")
//!         .await?; // CancellationError converts to anyhow::Error automatically
//!     Ok(result)
//! }
//! # }
//!
//! # async fn some_async_operation() -> Result<String, std::io::Error> {
//! #     Ok("test".to_string())
//! # }
//! ```

use log::info;
use std::future::Future;
use tokio_util::sync::CancellationToken;

/// Extension trait for adding cancellation support to any Future
///
/// This trait allows you to easily add cancellation support to any future that returns
/// a `Result`, eliminating the need for repetitive `tokio::select!` boilerplate code.
///
/// The trait is implemented for all futures that return `Result<T, E>` where both
/// the original error `E` and `CancellationError` can be converted to the target error type.
///
/// # Type Parameters
///
/// - `T`: The success type of the future's result
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use tokio_cancellation_ext::{CancellationExt, CancellationError};
/// use tokio_util::sync::CancellationToken;
///
/// #[derive(Debug)]
/// enum MyError {
///     Cancelled,
///     Other(String),
/// }
///
/// impl From<CancellationError> for MyError {
///     fn from(_: CancellationError) -> Self {
///         MyError::Cancelled
///     }
/// }
///
/// impl From<std::io::Error> for MyError {
///     fn from(e: std::io::Error) -> Self {
///         MyError::Other(e.to_string())
///     }
/// }
///
/// # async fn some_io_operation() -> Result<String, std::io::Error> {
/// #     Ok("success".to_string())
/// # }
/// # #[tokio::main]
/// # async fn main() -> Result<(), MyError> {
/// let token = CancellationToken::new();
///
/// let result = some_io_operation()
///     .with_cancellation::<MyError>(&token, "io_operation")
///     .await?;
///
/// println!("Got result: {}", result);
/// # Ok(())
/// # }
/// ```
///
/// ## Chaining Multiple Operations
///
/// ```rust
/// use tokio_cancellation_ext::{CancellationExt, CancellationError};
/// use tokio_util::sync::CancellationToken;
///
/// #[derive(Debug)]
/// enum AppError {
///     Cancelled,
///     Network(String),
///     Database(String),
/// }
///
/// impl From<CancellationError> for AppError {
///     fn from(_: CancellationError) -> Self {
///         AppError::Cancelled
///     }
/// }
///
/// # async fn fetch_user(id: u32) -> Result<String, AppError> { Ok("user".to_string()) }
/// # async fn update_cache(data: &str) -> Result<(), AppError> { Ok(()) }
/// # #[tokio::main]
/// # async fn main() -> Result<(), AppError> {
/// let token = CancellationToken::new();
/// let user_id = 42;
///
/// // Both operations can be cancelled
/// let user = fetch_user(user_id)
///     .with_cancellation(&token, "fetch_user")
///     .await?;
///
/// update_cache(&user)
///     .with_cancellation(&token, "update_cache")
///     .await?;
/// # Ok(())
/// # }
/// ```
pub trait CancellationExt<T> {
    /// The original error type from the future
    ///
    /// This associated type represents the error type that the original future
    /// can return. It must be convertible to the target error type `E`.
    type OriginalError;

    /// Adds cancellation support to a future
    ///
    /// This method wraps the future with cancellation logic, allowing it to be
    /// interrupted when the provided `CancellationToken` is cancelled.
    ///
    /// # Arguments
    ///
    /// * `token` - The cancellation token to listen for cancellation signals
    /// * `context` - A string describing the operation for logging purposes.
    ///   This will be included in log messages when cancellation occurs.
    ///
    /// # Returns
    ///
    /// Returns a new future that will either:
    /// - Complete with `Ok(T)` if the original future completes successfully
    /// - Complete with `Err(E)` if the original future returns an error (converted via `Into`)
    /// - Complete with `Err(E)` if cancellation is requested (from `CancellationError`)
    ///
    /// # Type Constraints
    ///
    /// - `CancellationError: Into<E>` - The cancellation error must be convertible to `E`
    /// - `Self::OriginalError: Into<E>` - The original error must be convertible to `E`
    /// - `Self: 'a` - The future must live at least as long as the references
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tokio_cancellation_ext::{CancellationExt, CancellationError};
    /// use tokio_util::sync::CancellationToken;
    ///
    /// #[derive(Debug)]
    /// enum DatabaseError {
    ///     Cancelled,
    ///     Connection(String),
    ///     Query(String),
    /// }
    ///
    /// impl From<CancellationError> for DatabaseError {
    ///     fn from(_: CancellationError) -> Self {
    ///         DatabaseError::Cancelled
    ///     }
    /// }
    ///
    /// # impl From<sqlx::Error> for DatabaseError {
    /// #     fn from(e: sqlx::Error) -> Self {
    /// #         DatabaseError::Query(e.to_string())
    /// #     }
    /// # }
    /// # async fn execute_query() -> Result<Vec<String>, DatabaseError> {
    /// #     Ok(vec!["row1".to_string(), "row2".to_string()])
    /// # }
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), DatabaseError> {
    /// let token = CancellationToken::new();
    ///
    /// let rows = execute_query()
    ///     .with_cancellation(&token, "execute_query")
    ///     .await?;
    ///
    /// println!("Got {} rows", rows.len());
    /// # Ok(())
    /// # }
    /// ```
    fn with_cancellation<'a, E>(
        self,
        token: &'a CancellationToken,
        context: &'a str,
    ) -> impl Future<Output = Result<T, E>> + Send + 'a
    where
        CancellationError: Into<E>,
        Self::OriginalError: Into<E>,
        Self: 'a;
}

/// Error type representing a cancellation request
///
/// This error is returned when an operation is cancelled via a `CancellationToken`.
/// It implements the standard `Error` trait and can be converted to other error types
/// via the `From`/`Into` traits.
///
/// # Examples
///
/// ## Converting to Custom Error Types
///
/// ```rust
/// use tokio_cancellation_ext::CancellationError;
///
/// #[derive(Debug)]
/// enum MyError {
///     Cancelled,
///     Network(String),
///     Database(String),
/// }
///
/// impl From<CancellationError> for MyError {
///     fn from(_: CancellationError) -> Self {
///         MyError::Cancelled
///     }
/// }
///
/// // Now CancellationError can be automatically converted to MyError
/// fn handle_error(err: CancellationError) -> MyError {
///     err.into() // Automatically converts to MyError::Cancelled
/// }
/// ```
///
/// ## Using with anyhow
///
/// ```rust
/// use tokio_cancellation_ext::CancellationError;
/// # #[cfg(feature = "anyhow")]
/// use anyhow::Result;
///
/// // CancellationError automatically converts to anyhow::Error
/// # #[cfg(feature = "anyhow")]
/// fn example() -> Result<()> {
///     let cancellation_err = CancellationError;
///     Err(cancellation_err.into()) // Works automatically
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CancellationError;

impl std::fmt::Display for CancellationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Operation was cancelled")
    }
}

impl std::error::Error for CancellationError {}

impl<F, T, OriginalError> CancellationExt<T> for F
where
    F: Future<Output = Result<T, OriginalError>> + Send,
{
    type OriginalError = OriginalError;

    fn with_cancellation<'a, E>(
        self,
        token: &'a CancellationToken,
        context: &'a str,
    ) -> impl Future<Output = Result<T, E>> + Send + 'a
    where
        CancellationError: Into<E>,
        OriginalError: Into<E>,
        F: 'a,
    {
        async move {
            let context_owned = context.to_string();
            tokio::select! {
                _ = token.cancelled() => {
                    info!("{}: cancellation signal received", context_owned);
                    Err(CancellationError.into())
                }
                result = self => {
                    result.map_err(Into::into)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[derive(Debug, PartialEq)]
    enum TestError {
        Cancelled,
        Custom(String),
    }

    impl From<CancellationError> for TestError {
        fn from(_: CancellationError) -> Self {
            TestError::Cancelled
        }
    }

    impl From<std::io::Error> for TestError {
        fn from(e: std::io::Error) -> Self {
            TestError::Custom(e.to_string())
        }
    }

    #[tokio::test]
    async fn test_successful_operation() {
        let token = CancellationToken::new();

        async fn success_operation() -> Result<String, std::io::Error> {
            Ok("success".to_string())
        }

        let result: Result<String, TestError> =
            success_operation().with_cancellation(&token, "test").await;

        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn test_cancellation() {
        let token = CancellationToken::new();

        async fn long_operation() -> Result<String, std::io::Error> {
            tokio::time::sleep(Duration::from_secs(10)).await;
            Ok("should not reach here".to_string())
        }

        // Cancel the token immediately
        token.cancel();

        let result: Result<String, TestError> = long_operation()
            .with_cancellation(&token, "test_cancellation")
            .await;

        assert_eq!(result.unwrap_err(), TestError::Cancelled);
    }

    #[tokio::test]
    async fn test_original_error_propagation() {
        let token = CancellationToken::new();

        async fn failing_operation() -> Result<String, std::io::Error> {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "test error"))
        }

        let result: Result<String, TestError> =
            failing_operation().with_cancellation(&token, "test").await;

        match result.unwrap_err() {
            TestError::Custom(msg) => assert!(msg.contains("test error")),
            _ => panic!("Expected Custom error"),
        }
    }
}
