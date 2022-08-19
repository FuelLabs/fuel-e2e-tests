#!/bin/bash

for folder in */; do
	folder_name="$(basename "$folder")"
	forc_name="$(grep -Po '(?<=name = ")[^"]+' "$folder/Forc.toml")";

	if [[ $forc_name != "$folder_name" ]]; then
		echo "$folder_name $forc_name"
	fi

done
