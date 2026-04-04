#!/bin/bash
BASE="https://archives.bulbagarden.net/media/upload/thumb"
declare -A icons=(
    ["normal"]="$BASE/8/89/Normal_icon_SV.png/48px-Normal_icon_SV.png"
    ["fire"]="$BASE/8/8f/Fire_icon_SV.png/48px-Fire_icon_SV.png"
    ["water"]="$BASE/3/34/Water_icon_SV.png/48px-Water_icon_SV.png"
    ["grass"]="$BASE/e/ee/Grass_icon_SV.png/48px-Grass_icon_SV.png"
    ["electric"]="$BASE/8/8e/Electric_icon_SV.png/48px-Electric_icon_SV.png"
    ["ice"]="$BASE/3/39/Ice_icon_SV.png/48px-Ice_icon_SV.png"
    ["fighting"]="$BASE/e/e5/Fighting_icon_SV.png/48px-Fighting_icon_SV.png"
    ["poison"]="$BASE/a/a7/Poison_icon_SV.png/48px-Poison_icon_SV.png"
    ["ground"]="$BASE/4/45/Ground_icon_SV.png/48px-Ground_icon_SV.png"
    ["flying"]="$BASE/a/ab/Flying_icon_SV.png/48px-Flying_icon_SV.png"
    ["psychic"]="$BASE/e/ee/Psychic_icon_SV.png/48px-Psychic_icon_SV.png"
    ["bug"]="$BASE/9/9a/Bug_icon_SV.png/48px-Bug_icon_SV.png"
    ["rock"]="$BASE/b/b2/Rock_icon_SV.png/48px-Rock_icon_SV.png"
    ["ghost"]="$BASE/c/c4/Ghost_icon_SV.png/48px-Ghost_icon_SV.png"
    ["dragon"]="$BASE/4/41/Dragon_icon_SV.png/48px-Dragon_icon_SV.png"
    ["dark"]="$BASE/3/3b/Dark_icon_SV.png/48px-Dark_icon_SV.png"
    ["steel"]="$BASE/e/e9/Steel_icon_SV.png/48px-Steel_icon_SV.png"
    ["fairy"]="$BASE/a/a9/Fairy_icon_SV.png/48px-Fairy_icon_SV.png"
)
for type in "${!icons[@]}"; do
    # O Bulbapedia proíbe curl sem o Header de User-Agent, então enviamos "Mozilla" para garantir
    curl -A "Mozilla/5.0" -s -L "${icons[$type]}" -o "static/icons/types/$type.png"
    sleep 0.5
done
