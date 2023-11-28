// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_binary::module_image::{
    data_index_section::{DataIndexEntry, DataIndexModuleEntry},
    external_func_index_section::{ExternalFuncIndexEntry, ExternalFuncIndexModuleEntry},
    func_index_section::{FuncIndexEntry, FuncIndexModuleEntry},
    unified_external_func_section::UnifiedExternalFuncEntry,
    unified_external_library_section::UnifiedExternalLibraryEntry,
};
use ancvm_types::DataSectionType;

use crate::{AssembleError, IndexEntry, ModuleEntry};

pub fn link(module_entries: &[&ModuleEntry]) -> Result<IndexEntry, AssembleError> {
    // todo
    // load shared modules

    // TEMPORARY, NO LINKING
    let func_index_module_entries = module_entries
        .iter()
        .enumerate()
        .map(|(module_index, module_entry)| {
            let entries = module_entry
                .func_entries
                .iter()
                .enumerate()
                .map(|(func_pub_index, _func_entry)| {
                    FuncIndexEntry::new(func_pub_index, module_index, func_pub_index)
                })
                .collect::<Vec<_>>();
            FuncIndexModuleEntry::new(entries)
        })
        .collect::<Vec<_>>();

    // TEMPORARY, NO LINKING
    // note that data internal index is section relevant.
    // e.g. there are indices 0,1,2,3... in read-only section, and
    // there are also indices 0,1,2,3... in read-write section.
    let data_index_module_entries = module_entries
        .iter()
        .enumerate()
        .map(|(module_index, module_entry)| {
            let mut data_pub_index = 0;
            let mut entries = vec![];

            for (data_internal_idx, _read_only_data_entry) in
                module_entry.read_only_data_entries.iter().enumerate()
            {
                entries.push(DataIndexEntry::new(
                    data_pub_index,
                    module_index,
                    data_internal_idx,
                    DataSectionType::ReadOnly,
                ));

                data_pub_index += 1;
            }

            for (data_internal_idx, _read_write_data_entry) in
                module_entry.read_write_data_entries.iter().enumerate()
            {
                entries.push(DataIndexEntry::new(
                    data_pub_index,
                    module_index,
                    data_internal_idx,
                    DataSectionType::ReadWrite,
                ));

                data_pub_index += 1;
            }

            for (data_internal_idx, _uninit_data_entry) in
                module_entry.uninit_data_entries.iter().enumerate()
            {
                entries.push(DataIndexEntry::new(
                    data_pub_index,
                    module_index,
                    data_internal_idx,
                    DataSectionType::Uninit,
                ));

                data_pub_index += 1;
            }

            DataIndexModuleEntry::new(entries)
        })
        .collect::<Vec<_>>();

    // linking external functions

    let mut unified_external_library_entries: Vec<UnifiedExternalLibraryEntry> = vec![];
    let mut unified_external_func_entries: Vec<UnifiedExternalFuncEntry> = vec![];
    let mut external_func_index_module_entries: Vec<ExternalFuncIndexModuleEntry> = vec![];

    for module_entry in module_entries {
        let mut external_func_index_entries: Vec<ExternalFuncIndexEntry> = vec![];

        for (original_external_library_index, external_library_entry) in
            module_entry.external_library_entries.iter().enumerate()
        {
            let unified_lib_idx_opt = unified_external_library_entries.iter().position(|entry| {
                entry.external_library_type == external_library_entry.external_library_type
                    && entry.name == external_library_entry.name
            });

            // add unified library entry when not found
            let unified_lib_idx = if let Some(idx) = unified_lib_idx_opt {
                idx
            } else {
                let idx = unified_external_library_entries.len();
                let unified_external_library_entry = UnifiedExternalLibraryEntry {
                    name: external_library_entry.name.clone(),
                    external_library_type: external_library_entry.external_library_type,
                };
                unified_external_library_entries.push(unified_external_library_entry);
                idx
            };

            // filter external functions by the specified external libray
            let external_func_entries_with_indices = module_entry
                .external_func_entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    entry.external_library_index == original_external_library_index
                })
                .collect::<Vec<_>>();

            for (original_external_func_index, external_func_entry) in
                external_func_entries_with_indices
            {
                let unified_func_idx_opt = unified_external_func_entries.iter().position(|entry| {
                    entry.unified_external_library_index == unified_lib_idx
                        && entry.name == external_func_entry.name
                });

                // add unified function entry when not found, and then
                // add external function index entry
                if unified_func_idx_opt.is_none() {
                    // add unified function entry
                    let unified_func_idx = unified_external_func_entries.len();
                    let unified_external_func_entry = UnifiedExternalFuncEntry {
                        name: external_func_entry.name.clone(),
                        unified_external_library_index: unified_lib_idx,
                    };
                    unified_external_func_entries.push(unified_external_func_entry);

                    // add external function index entry
                    let external_func_index_entry = ExternalFuncIndexEntry {
                        external_func_index: original_external_func_index,
                        unified_external_func_index: unified_func_idx,
                        type_index: external_func_entry.type_index,
                    };
                    external_func_index_entries.push(external_func_index_entry);
                }
            }
        }

        let external_func_index_module_entry = ExternalFuncIndexModuleEntry {
            index_entries: external_func_index_entries,
        };
        external_func_index_module_entries.push(external_func_index_module_entry);
    }

    Ok(IndexEntry {
        func_index_module_entries,
        data_index_module_entries,
        unified_external_library_entries,
        unified_external_func_entries,
        external_func_index_module_entries,
    })
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_link() {
        // todo
    }
}
