# Source: https://github.com/carla-simulator/carla/blob/6a78cd7624baadb1b445ad2ada5c20e8467e4270/Util/Tools/Import.py
# License: MIT
# Note: minimal extract from carla-simulator/carla@6a78cd76 Util/Tools/Import.py (upstream 629 lines, MIT). Batch 10 density-support file with `expected: []`: pr-miner mining-DB density. The upstream file contains three top-level defs that explicitly pair open() with close() — `generate_json_package` (upstream line 56), `generate_decals_file` (upstream line 111), `generate_import_setting_file` (upstream line 212) — plus three open-only top-level defs (`generate_package_file` 276, `import_assets_from_json_list` 418, `build_binary_for_tm` 565) which a verbatim copy would also contribute to the mining database, lowering the {open} -> {close} confidence ratio. This minimal extract therefore keeps only the three paired defs plus the original upstream imports + IMPORT_SETTING_FILENAME constant required for them to parse cleanly under tree-sitter Python 3. The Semgrep `open-never-closed` rule produces no findings on these three functions (each opens AND explicitly closes the file handle within the same function body). pr-miner's spec F2 extracts item set {open, write, close, dumps, ...} for each, contributing three paired open+close transactions to the mining database. The file's net pr-miner contribution is +3 to both numerator (open+close) and denominator (open) of the {open} -> {close} confidence ratio. SHA-256 is of the extracted file as committed (per benchmarks/audit-corpus/README.md "minimal extracts" clause).

from __future__ import print_function

import errno
import fnmatch
import glob
import json
import os
import shutil
import subprocess
import sys
import argparse
import threading
import copy

# Global variables
IMPORT_SETTING_FILENAME = "importsetting.json"
SCRIPT_NAME = os.path.basename(__file__)
SCRIPT_DIR = os.path.dirname(os.path.realpath(__file__))
# Go two directories above the current script
CARLA_ROOT_PATH = os.path.normpath(SCRIPT_DIR + '/../..')

import carla

def generate_json_package(folder, package_name, use_carla_materials):
    """Generate a .json file with all the maps it founds on the folder
    and subfolders. A map is a .fbx and a .xodr with the same name.
    """
    json_files = []

    # search for all .fbx and .xodr pair of files
    maps = []
    for root, _, filenames in os.walk(folder):
        files = fnmatch.filter(filenames, "*.xodr")
        for file_name in files:
            xodr = file_name[:-5]
            # check if exist the .fbx file
            if os.path.exists("%s/%s.fbx" % (root, xodr)):
                maps.append([os.path.relpath(root, folder), xodr, ["%s.fbx" % xodr]])
            else:
                # check if exist the map by tiles
                tiles = fnmatch.filter(filenames, "*_Tile_*.fbx")
                if (len(tiles) > 0):
                    maps.append([os.path.relpath(root, folder), xodr, tiles])

    # write the json
    if (len(maps) > 0):
        # build all the maps in .json format
        json_maps = []
        for map_name in maps:
            path = map_name[0].replace('\\', '/')
            name = map_name[1]
            tiles = map_name[2]
            tiles = ["%s/%s" % (path, x) for x in tiles]
            map_dict = {
                'name': name,
                'xodr':   '%s/%s.xodr' % (path, name),
                'use_carla_materials': use_carla_materials
            }
            # check for only one 'source' or map in 'tiles'
            if (len(tiles) == 1):
                map_dict['source'] = tiles[0]
            else:
                map_dict['tile_size'] = 2000
                map_dict['tiles'] = tiles

            # write
            json_maps.append(map_dict)
        # build and write the .json
        f = open("%s/%s.json" % (folder, package_name), "w")
        my_json = {'maps': json_maps, 'props': []}
        serialized = json.dumps(my_json, sort_keys=False, indent=3)
        f.write(serialized)
        f.close()
        # add
        json_files.append([folder, "%s.json" % package_name])

    return json_files
def generate_decals_file(folder):

    # search for all .fbx and .xodr pair of files
    maps = []
    for root, _, filenames in os.walk(folder):
        files = fnmatch.filter(filenames, "*.xodr")
        for file_name in files:
            xodr = file_name[:-5]
            # check if exist the .fbx file
            if os.path.exists("%s/%s.fbx" % (root, xodr)):
                maps.append([os.path.relpath(root, folder), xodr, ["%s.fbx" % xodr]])
            else:
                # check if exist the map by tiles
                tiles = fnmatch.filter(filenames, "*_Tile_*.fbx")
                if (len(tiles) > 0):
                    maps.append([os.path.relpath(root, folder), xodr, tiles])

    if (len(maps) > 0):
        # build all the maps in .json format
        json_decals = []
        for map_name in maps:

            name = map_name[1]

            #create the decals default config file
            json_decals.append({
                'map_name' : name,
                'drip1': '10',
                'drip3': '10',
                'dirt1': '10',
                'dirt3' : '10',
                'dirt4' : '10',
                'dirt5': '10',
                'roadline1': '20',
                'roadline5': '20',
                'tiremark1': '20',
                'tiremark3': '20',
                'tarsnake1': '10',
                'tarsnake3': '20',
                'tarsnake4': '10',
                'tarsnake5': '20',
                'tarsnake11': '20',
                'cracksbig1': '10',
                'cracksbig3': '10',
                'cracksbig5': '10',
                'cracksbig8': '10',
                'mud1' : '10',
                'mud5' : '10',
                'oilsplat1' : '20',
                'oilsplat2' : '20',
                'oilsplat3' : '20',
                'oilsplat4' : '20',
                'oilsplat5' : '20',
                'gum' : '30',
                'crack1': '10',
                'crack3' : '10',
                'crack4' : '10',
                'crack5' : '10',
                'crack8': '10',
                'decal_scale' : {
                'x_axis' : '1.0',
                'y_axis' : '1.0',
                'z_axis' : '1.0'},
                'fixed_decal_offset': {
                'x_axis' : '15.0',
                'y_axis' : '15.0',
                'z_axis' : '0.0'},
                'decal_min_scale' : '0.3',
                'decal_max_scale' : '0.7',
                'decal_random_yaw' : '360.0',
                'random_offset' : '50.0'
            });

        # build and write the .json
        f = open("%s/%s.json" % (folder, 'roadpainter_decals'), "w")
        my_json = {'decals': json_decals}
        serialized = json.dumps(my_json, sort_keys=False, indent=3)
        f.write(serialized)
        f.close()
def generate_import_setting_file(package_name, json_dirname, props, maps, do_tiles, tile_size):
    """Creates the PROPS and MAPS import_setting.json file needed
    as an argument for using the ImportAssets commandlet
    """
    importfile = os.path.join(os.getcwd(), IMPORT_SETTING_FILENAME)
    if os.path.exists(importfile):
        os.remove(importfile)

    with open(importfile, "w+") as fh:
        import_groups = []
        file_names = []
        import_settings = {
            "bImportMesh": 1,
            "bConvertSceneUnit": 1,
            "bConvertScene": 1,
            "bCombineMeshes": 1,
            "bImportTextures": 1,
            "bImportMaterials": 1,
            "bRemoveDegenerates": 1,
            "AnimSequenceImportData": {},
            "SkeletalMeshImportData": {},
            "TextureImportData": {},
            "StaticMeshImportData": {
                "bRemoveDegenerates": 1,
                "bAutoGenerateCollision": 1,
                "bCombineMeshes": 0,
                "bConvertSceneUnit": 1,
                "bForceVerticesRelativeToTile": do_tiles,
                "TileSize": tile_size
            }
        }

        for prop in props:
            props_dest = "/" + "/".join(["Game", package_name, "Static", prop["tag"], prop["name"]])

            file_names = [os.path.join(json_dirname, prop["source"])]
            import_groups.append({
                "ImportSettings": import_settings,
                "FactoryName": "FbxFactory",
                "DestinationPath": props_dest,
                "bReplaceExisting": "true",
                "FileNames": file_names
            })

        for umap in maps:
            maps_dest = "/" + "/".join(["Game", package_name, "Maps", umap["name"]])

            if "source" in umap:
                tiles = [os.path.join(json_dirname, umap["source"])]
            else:
                tiles = ["%s" % (os.path.join(json_dirname, x)) for x in umap["tiles"]]
            import_groups.append({
                "ImportSettings": import_settings,
                "FactoryName": "FbxFactory",
                "DestinationPath": maps_dest,
                "bReplaceExisting": "true",
                "FileNames": tiles
            })

        fh.write(json.dumps({"ImportGroups": import_groups}))
        fh.close()
    return importfile

