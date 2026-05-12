# Source: https://github.com/wasserth/TotalSegmentator/blob/812adff1b80e9ffb7a11a9acf210b868f7a5c66f/totalsegmentator/statistics.py
# License: Apache-2.0
# Note: minimal extract reproducing the swapped-args call at upstream line 58; fixed in PR #556 by reordering the call to get_radiomics_features(mask, ct_file)
def get_radiomics_features(seg_file, img_file="ct.nii.gz"):
    return seg_file, img_file


def get_radiomics_features_for_entire_dir(ct_file, mask_dir, file_out):
    masks = sorted(list(mask_dir.glob("*.nii.gz")))
    stats = [get_radiomics_features(ct_file, mask) for mask in masks]
    stats = {mask_name: stats for mask_name, stats in stats}
