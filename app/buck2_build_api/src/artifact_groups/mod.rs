/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

mod artifact_group_values;
pub mod calculation;
pub mod deferred;
pub mod promise;

use crate::interpreter::rule_defs::context::get_artifact_from_anon_target_analysis;

pub mod registry;

use std::hash::Hash;

use allocative::Allocative;
pub use artifact_group_values::ArtifactGroupValues;
use buck2_artifact::artifact::artifact_type::Artifact;
use derive_more::Display;
use dice::DiceComputations;
use dupe::Dupe;
use gazebo::variants::UnpackVariants;

use crate::artifact_groups::deferred::TransitiveSetKey;
use crate::artifact_groups::promise::PromiseArtifact;

/// An [ArtifactGroup] can expand to one or more [Artifact]. Those Artifacts will be made available
/// to Actions when they execute.
#[derive(
    Clone,
    Debug,
    Display,
    Dupe,
    PartialEq,
    Eq,
    Hash,
    UnpackVariants,
    Allocative
)]
pub enum ArtifactGroup {
    Artifact(Artifact),
    TransitiveSetProjection(TransitiveSetProjectionKey),
    Promise(PromiseArtifact),
}

impl ArtifactGroup {
    // TODO(@wendyy) - deprecate
    pub fn assert_resolved(&self) -> ResolvedArtifactGroup {
        self.resolved().unwrap()
    }

    // TODO(@wendyy) - deprecate
    pub fn resolved(&self) -> anyhow::Result<ResolvedArtifactGroup> {
        Ok(match self {
            ArtifactGroup::Artifact(a) => ResolvedArtifactGroup::Artifact(a.clone()),
            ArtifactGroup::TransitiveSetProjection(a) => {
                ResolvedArtifactGroup::TransitiveSetProjection(a)
            }
            ArtifactGroup::Promise(p) => ResolvedArtifactGroup::Artifact(p.get_err()?.clone()),
        })
    }

    /// Gets the resolved artifact group, which is used further downstream to use DICE to get
    /// or compute the actual artifact values. For the `Artifact` variant, we will get the results
    /// via the base or projected artifact key. For the `TransitiveSetProjection` variant, we will
    /// look get the resutls via the `EnsureTransitiveSetProjectionKey`, which expands the underlying
    /// tset. For the `Promise` variant, we will look up the promised artifact values by getting
    /// the analysis results of the owning anon target's analysis.
    pub async fn resolved_artifact(
        &self,
        ctx: &DiceComputations,
    ) -> anyhow::Result<ResolvedArtifactGroup> {
        Ok(match self {
            ArtifactGroup::Artifact(a) => ResolvedArtifactGroup::Artifact(a.clone()),
            ArtifactGroup::TransitiveSetProjection(a) => {
                ResolvedArtifactGroup::TransitiveSetProjection(a)
            }
            ArtifactGroup::Promise(p) => match p.get() {
                Some(a) => ResolvedArtifactGroup::Artifact(a.clone()),
                None => {
                    let artifact = get_artifact_from_anon_target_analysis(p, ctx).await?;
                    ResolvedArtifactGroup::Artifact(artifact)
                }
            },
        })
    }
}

// TODO(@wendyy) if we move PromiseArtifact into ArtifactKind someday, we should probably
// split the Artifact variant into two cases (artifact by ref and by value) to prevent memory
// regressions.
pub enum ResolvedArtifactGroup<'a> {
    Artifact(Artifact),
    TransitiveSetProjection(&'a TransitiveSetProjectionKey),
}

#[derive(Clone, Debug, Display, Dupe, PartialEq, Eq, Hash, Allocative)]
#[display(fmt = "TransitiveSetProjection({}, {})", key, projection)]
pub struct TransitiveSetProjectionKey {
    pub key: TransitiveSetKey,
    pub projection: usize,
}
