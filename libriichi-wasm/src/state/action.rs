use super::PlayerState;
use crate::chi_type::ChiType;
use crate::mjai::Event;
use crate::tile::Tile;
use crate::tuz;

use anyhow::{Result, bail, ensure};
use serde::Serialize;

#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct ActionCandidate {
    pub can_discard: bool,
    pub can_chi_low: bool,
    pub can_chi_mid: bool,
    pub can_chi_high: bool,
    pub can_pon: bool,
    pub can_daiminkan: bool,
    pub can_kakan: bool,
    pub can_ankan: bool,
    pub can_riichi: bool,
    pub can_tsumo_agari: bool,
    pub can_ron_agari: bool,
    pub can_ryukyoku: bool,

    pub can_nukidora: bool,

    pub target_actor: u8,
}

impl ActionCandidate {
    #[inline]
    #[must_use]
    pub const fn can_chi(&self) -> bool {
        self.can_chi_low || self.can_chi_mid || self.can_chi_high
    }

    #[inline]
    #[must_use]
    pub const fn can_kan(&self) -> bool {
        self.can_daiminkan || self.can_kakan || self.can_ankan
    }

    #[inline]
    #[must_use]
    pub const fn can_agari(&self) -> bool {
        self.can_tsumo_agari || self.can_ron_agari
    }

    #[inline]
    #[must_use]
    pub const fn can_pass(&self) -> bool {
        self.can_chi() || self.can_pon || self.can_daiminkan || self.can_ron_agari
    }

    #[inline]
    #[must_use]
    pub const fn can_act(&self) -> bool {
        self.can_discard
            || self.can_chi()
            || self.can_pon
            || self.can_kan()
            || self.can_riichi
            || self.can_agari()
            || self.can_ryukyoku
            || self.can_nukidora
    }
}

impl PlayerState {
    pub fn validate_reaction(&self, action: &Event) -> Result<()> {
        let cans = self.last_cans;

        match action {
            Event::Ryukyoku { .. } => {
                ensure!(cans.can_ryukyoku, "cannot ryukyoku");
                return Ok(());
            }
            Event::None => {
                return Ok(());
            }
            _ => (),
        };

        if let Some(actor) = action.actor() {
            ensure!(
                actor == self.player_id,
                "actor is {actor}, not self ({})",
                self.player_id,
            );
        } else {
            bail!("action does not have actor and is not ryukyoku");
        }

        match *action {
            Event::Dahai { pai, tsumogiri, .. } => {
                ensure!(cans.can_discard, "cannot discard");
                self.ensure_tiles_in_hand(&[pai])?;
                if tsumogiri {
                    if let Some(tile) = self.last_self_tsumo {
                        ensure!(tile == pai, "cannot tsumogiri");
                    } else {
                        bail!("tsumogiri but the player has not dealt any tile yet");
                    }
                }
            }

            Event::Reach { .. } => {
                ensure!(cans.can_riichi, "cannot riichi");
            }

            Event::Chi {
                actor,
                target,
                pai,
                consumed,
            } => {
                ensure!((target + 1) % 3 == actor, "chi from non-kamicha");
                ensure!(
                    matches!(self.last_kawa_tile, Some(tile) if tile == pai),
                    "chi target is not the last kawa tile",
                );
                self.ensure_tiles_in_hand(&consumed)?;

                match ChiType::new(consumed, pai) {
                    ChiType::Low => ensure!(cans.can_chi_low, "cannot chi low"),
                    ChiType::Mid => ensure!(cans.can_chi_mid, "cannot chi mid"),
                    ChiType::High => ensure!(cans.can_chi_high, "cannot chi high"),
                }
            }
            Event::Pon {
                actor,
                target,
                pai,
                consumed,
            } => {
                ensure!(target != actor, "pon from itself");
                ensure!(
                    matches!(self.last_kawa_tile, Some(tile) if tile == pai),
                    "pon target is not the last kawa tile",
                );
                ensure!(cans.can_pon, "cannot pon");
                self.ensure_tiles_in_hand(&consumed)?;
            }

            Event::Daiminkan {
                actor,
                target,
                pai,
                consumed,
            } => {
                ensure!(target != actor, "daiminkan from itself");
                ensure!(
                    matches!(self.last_kawa_tile, Some(tile) if tile == pai),
                    "daiminkan target is not the last kawa tile",
                );
                ensure!(cans.can_daiminkan, "cannot daiminkan");
                self.ensure_tiles_in_hand(&consumed)?;
            }
            Event::Kakan { pai, .. } => {
                ensure!(cans.can_kakan, "cannot kakan");
                ensure!(
                    self.kakan_candidates.contains(&pai.deaka()),
                    "cannot kakan {pai}",
                );
                self.ensure_tiles_in_hand(&[pai])?;
            }
            Event::Ankan { consumed, .. } => {
                ensure!(cans.can_ankan, "cannot ankan");
                let tile = consumed[0].deaka();
                ensure!(self.ankan_candidates.contains(&tile), "cannot ankan {tile}");
                self.ensure_tiles_in_hand(&consumed)?;
            }

            Event::Hora { target, .. } => {
                if target == self.player_id {
                    ensure!(cans.can_tsumo_agari, "cannot tsumo agari");
                } else {
                    ensure!(cans.can_ron_agari, "cannot ron agari");
                }
            }

            Event::Nukidora { pai, .. } => {
                ensure!(cans.can_nukidora, "cannot nukidora");
                self.ensure_tiles_in_hand(&[pai])?;
            }

            Event::None => return Ok(()),

            _ => bail!("unexpected action {action:?}"),
        };

        Ok(())
    }

    fn ensure_tiles_in_hand(&self, tiles: &[Tile]) -> Result<()> {
        for &tile in tiles {
            ensure!(
                self.tehai[tile.deaka().as_usize()] > 0,
                "{tile} is not in hand",
            );
            if tile.is_aka() {
                ensure!(
                    self.akas_in_hand[tile.as_usize() - tuz!(5mr)],
                    "{tile} is not in hand",
                );
            }
        }
        Ok(())
    }
}
