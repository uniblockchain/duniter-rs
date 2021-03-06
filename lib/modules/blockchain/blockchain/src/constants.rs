//  Copyright (C) 2018  The Durs Project Developers.
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

/// Module name
pub static MODULE_NAME: &'static str = "blockchain";

/// Default currency
pub static DEFAULT_CURRENCY: &'static str = "g1";

/// Chunk size (in blocks)
pub static CHUNK_SIZE: &'static usize = &250;

/// Chunk file name begin
pub static CHUNK_FILE_NAME_BEGIN: &'static str = "chunk_";

/// Chunk file name end
pub static CHUNK_FILE_NAME_END: &'static str = "-250.json";
