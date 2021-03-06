//  Copyright (C) 2018  The Duniter Project Developers.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Durs-core cli : dbex subcommands.

use crate::commands::DursExecutableCoreCommand;
use crate::dbex;
use crate::errors::DursCoreError;
use crate::DursCore;
use durs_blockchain::{DBExQuery, DBExTxQuery, DBExWotQuery};
use durs_conf::DuRsConf;

#[derive(StructOpt, Debug, Clone)]
#[structopt(
    name = "dbex",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
/// durs databases explorer
pub struct DbExOpt {
    #[structopt(short = "c", long = "csv")]
    /// csv output
    pub csv: bool,
    #[structopt(subcommand)]
    /// DbExSubCommand
    pub subcommand: DbExSubCommand,
}

#[derive(StructOpt, Debug, Clone)]
/// dbex subcommands
pub enum DbExSubCommand {
    #[structopt(
        name = "distance",
        raw(setting = "structopt::clap::AppSettings::ColoredHelp")
    )]
    /// Web of Trust distances explorer
    DistanceOpt(DistanceOpt),
    #[structopt(
        name = "members",
        raw(setting = "structopt::clap::AppSettings::ColoredHelp")
    )]
    /// Members explorer
    MembersOpt(MembersOpt),
    #[structopt(
        name = "member",
        raw(setting = "structopt::clap::AppSettings::ColoredHelp")
    )]
    /// Member explorer
    MemberOpt(MemberOpt),
    #[structopt(
        name = "balance",
        raw(setting = "structopt::clap::AppSettings::ColoredHelp")
    )]
    /// Pubkeys’ balances explorer
    BalanceOpt(BalanceOpt),
}

#[derive(StructOpt, Debug, Copy, Clone)]
/// DistanceOpt
pub struct DistanceOpt {
    #[structopt(short = "r", long = "reverse")]
    /// reverse order
    pub reverse: bool,
}

#[derive(StructOpt, Debug, Copy, Clone)]
/// MembersOpt
pub struct MembersOpt {
    #[structopt(short = "r", long = "reverse")]
    /// reverse order
    pub reverse: bool,
    #[structopt(short = "e", long = "expire")]
    /// show members expire date
    pub expire: bool,
}

#[derive(StructOpt, Debug, Clone)]
/// MemberOpt
pub struct MemberOpt {
    /// choose member uid
    pub uid: String,
}

#[derive(StructOpt, Debug, Clone)]
/// BalanceOpt
pub struct BalanceOpt {
    /// public key or uid
    pub address: String,
}

impl DursExecutableCoreCommand for DbExOpt {
    fn execute(self, durs_core: DursCore<DuRsConf>) -> Result<(), DursCoreError> {
        let profile_path = durs_core.soft_meta_datas.profile_path;

        match self.subcommand {
            DbExSubCommand::DistanceOpt(distance_opts) => dbex(
                profile_path,
                &durs_core.soft_meta_datas.conf,
                self.csv,
                &DBExQuery::WotQuery(DBExWotQuery::AllDistances(distance_opts.reverse)),
            ),
            DbExSubCommand::MemberOpt(member_opts) => dbex(
                profile_path,
                &durs_core.soft_meta_datas.conf,
                self.csv,
                &DBExQuery::WotQuery(DBExWotQuery::MemberDatas(member_opts.uid)),
            ),
            DbExSubCommand::MembersOpt(members_opts) => {
                if members_opts.expire {
                    dbex(
                        profile_path,
                        &durs_core.soft_meta_datas.conf,
                        self.csv,
                        &DBExQuery::WotQuery(DBExWotQuery::ExpireMembers(members_opts.reverse)),
                    );
                } else {
                    dbex(
                        profile_path,
                        &durs_core.soft_meta_datas.conf,
                        self.csv,
                        &DBExQuery::WotQuery(DBExWotQuery::ListMembers(members_opts.reverse)),
                    );
                }
            }
            DbExSubCommand::BalanceOpt(balance_opts) => dbex(
                profile_path,
                &durs_core.soft_meta_datas.conf,
                self.csv,
                &DBExQuery::TxQuery(DBExTxQuery::Balance(balance_opts.address)),
            ),
        }

        Ok(())
    }
}
