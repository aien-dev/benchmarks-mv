// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Drake Stapleton (AIEN / aien-dev)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! AIEN Prime Drag Race suite runner. See CONTRACT.md next to this crate.
//!
//! Std plus serde_json, sha2, chrono and clap (all already in the parent Cargo.lock).

pub mod exec;
pub mod receipt;
pub mod reference;
pub mod runner;
pub mod sweep;
pub mod validate;

/// Frozen contract version (see prime-drag-race/CONTRACT.md).
pub const CONTRACT_VERSION: &str = "1";
pub const REPORT_SCHEMA: &str = "aien-prime-race/impl-report/v1";
pub const RECEIPT_SCHEMA: &str = "aien-prime-race/receipt/v1";
