CREATE TABLE `abilities` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`is_main_series` integer DEFAULT true NOT NULL,
	`generation_id` integer,
	FOREIGN KEY (`generation_id`) REFERENCES `generations`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE UNIQUE INDEX `abilities_name_unique` ON `abilities` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `abilities_slug_unique` ON `abilities` (`slug`);--> statement-breakpoint
CREATE TABLE `ability_effects` (
	`ability_id` integer NOT NULL,
	`effect` text NOT NULL,
	`short_effect` text NOT NULL,
	`language_id` integer NOT NULL,
	PRIMARY KEY(`ability_id`, `language_id`),
	FOREIGN KEY (`ability_id`) REFERENCES `abilities`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `ability_flavor_texts` (
	`ability_id` integer NOT NULL,
	`flavor_text` text NOT NULL,
	`language_id` integer NOT NULL,
	`version_group_id` integer NOT NULL,
	PRIMARY KEY(`ability_id`, `language_id`, `version_group_id`),
	FOREIGN KEY (`ability_id`) REFERENCES `abilities`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`version_group_id`) REFERENCES `version_groups`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `ability_names` (
	`ability_id` integer NOT NULL,
	`language_id` integer NOT NULL,
	`name` text NOT NULL,
	PRIMARY KEY(`ability_id`, `language_id`),
	FOREIGN KEY (`ability_id`) REFERENCES `abilities`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `move_effects` (
	`move_id` integer NOT NULL,
	`effect` text NOT NULL,
	`short_effect` text NOT NULL,
	`language_id` integer NOT NULL,
	PRIMARY KEY(`move_id`, `language_id`),
	FOREIGN KEY (`move_id`) REFERENCES `moves`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `move_flavor_texts` (
	`move_id` integer NOT NULL,
	`flavor_text` text NOT NULL,
	`language_id` integer NOT NULL,
	`version_group_id` integer NOT NULL,
	PRIMARY KEY(`move_id`, `language_id`, `version_group_id`),
	FOREIGN KEY (`move_id`) REFERENCES `moves`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`version_group_id`) REFERENCES `version_groups`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `move_learn_method_names` (
	`move_learn_method_id` integer NOT NULL,
	`language_id` integer NOT NULL,
	`name` text NOT NULL,
	`description` text,
	PRIMARY KEY(`move_learn_method_id`, `language_id`),
	FOREIGN KEY (`move_learn_method_id`) REFERENCES `move_learn_methods`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `move_learn_methods` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`description` text
);
--> statement-breakpoint
CREATE UNIQUE INDEX `move_learn_methods_name_unique` ON `move_learn_methods` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `move_learn_methods_slug_unique` ON `move_learn_methods` (`slug`);--> statement-breakpoint
CREATE TABLE `move_names` (
	`move_id` integer NOT NULL,
	`language_id` integer NOT NULL,
	`name` text NOT NULL,
	PRIMARY KEY(`move_id`, `language_id`),
	FOREIGN KEY (`move_id`) REFERENCES `moves`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `moves` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`type_id` integer NOT NULL,
	`damage_class` text NOT NULL,
	`power` integer,
	`accuracy` integer,
	`pp` integer NOT NULL,
	`priority` integer DEFAULT 0 NOT NULL,
	`target` text,
	`generation_id` integer,
	FOREIGN KEY (`type_id`) REFERENCES `types`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`generation_id`) REFERENCES `generations`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE UNIQUE INDEX `moves_name_unique` ON `moves` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `moves_slug_unique` ON `moves` (`slug`);--> statement-breakpoint
CREATE TABLE `type_effectiveness` (
	`attacker_id` integer NOT NULL,
	`defender_id` integer NOT NULL,
	`multiplier` real NOT NULL,
	PRIMARY KEY(`attacker_id`, `defender_id`),
	FOREIGN KEY (`attacker_id`) REFERENCES `types`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`defender_id`) REFERENCES `types`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `type_names` (
	`type_id` integer NOT NULL,
	`language_id` integer NOT NULL,
	`name` text NOT NULL,
	PRIMARY KEY(`type_id`, `language_id`),
	FOREIGN KEY (`type_id`) REFERENCES `types`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `types` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`hexcolor` text,
	`icon_url` text,
	`move_damage_class` text,
	`generation_id` integer,
	FOREIGN KEY (`generation_id`) REFERENCES `generations`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE UNIQUE INDEX `types_name_unique` ON `types` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `types_slug_unique` ON `types` (`slug`);--> statement-breakpoint
CREATE TABLE `generation_names` (
	`generation_id` integer NOT NULL,
	`language_id` integer NOT NULL,
	`name` text NOT NULL,
	PRIMARY KEY(`generation_id`, `language_id`),
	FOREIGN KEY (`generation_id`) REFERENCES `generations`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `generations` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`region_id` integer,
	`order` integer,
	FOREIGN KEY (`region_id`) REFERENCES `regions`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE UNIQUE INDEX `generations_name_unique` ON `generations` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `generations_slug_unique` ON `generations` (`slug`);--> statement-breakpoint
CREATE TABLE `languages` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`name` text NOT NULL,
	`iso639` text,
	`iso3166` text,
	`is_official` integer DEFAULT true NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `languages_name_unique` ON `languages` (`name`);--> statement-breakpoint
CREATE TABLE `pokedex_names` (
	`pokedex_id` integer NOT NULL,
	`language_id` integer NOT NULL,
	`name` text NOT NULL,
	`description` text,
	PRIMARY KEY(`pokedex_id`, `language_id`),
	FOREIGN KEY (`pokedex_id`) REFERENCES `pokedexes`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `pokedexes` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`region_id` integer,
	`is_main_series` integer DEFAULT true NOT NULL,
	FOREIGN KEY (`region_id`) REFERENCES `regions`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE UNIQUE INDEX `pokedexes_name_unique` ON `pokedexes` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `pokedexes_slug_unique` ON `pokedexes` (`slug`);--> statement-breakpoint
CREATE TABLE `region_names` (
	`region_id` integer NOT NULL,
	`language_id` integer NOT NULL,
	`name` text NOT NULL,
	PRIMARY KEY(`region_id`, `language_id`),
	FOREIGN KEY (`region_id`) REFERENCES `regions`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `regions` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`main_generation_id` integer
);
--> statement-breakpoint
CREATE UNIQUE INDEX `regions_name_unique` ON `regions` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `regions_slug_unique` ON `regions` (`slug`);--> statement-breakpoint
CREATE TABLE `version_group_names` (
	`version_group_id` integer NOT NULL,
	`language_id` integer NOT NULL,
	`name` text NOT NULL,
	PRIMARY KEY(`version_group_id`, `language_id`),
	FOREIGN KEY (`version_group_id`) REFERENCES `version_groups`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `version_groups` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`generation_id` integer NOT NULL,
	`order` integer,
	FOREIGN KEY (`generation_id`) REFERENCES `generations`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE UNIQUE INDEX `version_groups_name_unique` ON `version_groups` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `version_groups_slug_unique` ON `version_groups` (`slug`);--> statement-breakpoint
CREATE TABLE `version_names` (
	`version_id` integer NOT NULL,
	`language_id` integer NOT NULL,
	`name` text NOT NULL,
	PRIMARY KEY(`version_id`, `language_id`),
	FOREIGN KEY (`version_id`) REFERENCES `versions`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `versions` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`version_group_id` integer NOT NULL,
	FOREIGN KEY (`version_group_id`) REFERENCES `version_groups`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE UNIQUE INDEX `versions_name_unique` ON `versions` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `versions_slug_unique` ON `versions` (`slug`);--> statement-breakpoint
CREATE TABLE `encounter_condition_values` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`condition_id` integer NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	FOREIGN KEY (`condition_id`) REFERENCES `encounter_conditions`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE UNIQUE INDEX `encounter_condition_values_name_unique` ON `encounter_condition_values` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `encounter_condition_values_slug_unique` ON `encounter_condition_values` (`slug`);--> statement-breakpoint
CREATE TABLE `encounter_conditions` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`name` text NOT NULL,
	`slug` text NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `encounter_conditions_name_unique` ON `encounter_conditions` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `encounter_conditions_slug_unique` ON `encounter_conditions` (`slug`);--> statement-breakpoint
CREATE TABLE `encounter_method_names` (
	`encounter_method_id` integer NOT NULL,
	`language_id` integer NOT NULL,
	`name` text NOT NULL,
	`description` text,
	PRIMARY KEY(`encounter_method_id`, `language_id`),
	FOREIGN KEY (`encounter_method_id`) REFERENCES `encounter_methods`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `encounter_methods` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`order` integer
);
--> statement-breakpoint
CREATE UNIQUE INDEX `encounter_methods_name_unique` ON `encounter_methods` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `encounter_methods_slug_unique` ON `encounter_methods` (`slug`);--> statement-breakpoint
CREATE TABLE `location_area_names` (
	`location_area_id` integer NOT NULL,
	`language_id` integer NOT NULL,
	`name` text NOT NULL,
	PRIMARY KEY(`location_area_id`, `language_id`),
	FOREIGN KEY (`location_area_id`) REFERENCES `location_areas`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `location_areas` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`location_id` integer NOT NULL,
	`game_index` integer,
	FOREIGN KEY (`location_id`) REFERENCES `locations`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE UNIQUE INDEX `location_areas_name_unique` ON `location_areas` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `location_areas_slug_unique` ON `location_areas` (`slug`);--> statement-breakpoint
CREATE TABLE `location_names` (
	`location_id` integer NOT NULL,
	`language_id` integer NOT NULL,
	`name` text NOT NULL,
	PRIMARY KEY(`location_id`, `language_id`),
	FOREIGN KEY (`location_id`) REFERENCES `locations`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `locations` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`region_id` integer,
	FOREIGN KEY (`region_id`) REFERENCES `regions`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE UNIQUE INDEX `locations_name_unique` ON `locations` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `locations_slug_unique` ON `locations` (`slug`);--> statement-breakpoint
CREATE TABLE `item_effects` (
	`item_id` integer NOT NULL,
	`effect` text NOT NULL,
	`short_effect` text NOT NULL,
	`language_id` integer NOT NULL,
	PRIMARY KEY(`item_id`, `language_id`),
	FOREIGN KEY (`item_id`) REFERENCES `items`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `item_flavor_texts` (
	`item_id` integer NOT NULL,
	`flavor_text` text NOT NULL,
	`language_id` integer NOT NULL,
	`version_group_id` integer NOT NULL,
	PRIMARY KEY(`item_id`, `language_id`, `version_group_id`),
	FOREIGN KEY (`item_id`) REFERENCES `items`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`version_group_id`) REFERENCES `version_groups`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `item_names` (
	`item_id` integer NOT NULL,
	`language_id` integer NOT NULL,
	`name` text NOT NULL,
	PRIMARY KEY(`item_id`, `language_id`),
	FOREIGN KEY (`item_id`) REFERENCES `items`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `items` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`cost` integer,
	`fling_power` integer,
	`category` text,
	`sprite_url` text
);
--> statement-breakpoint
CREATE UNIQUE INDEX `items_name_unique` ON `items` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `items_slug_unique` ON `items` (`slug`);--> statement-breakpoint
CREATE TABLE `natures` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`increased_stat` text,
	`decreased_stat` text,
	`likes_flavor` text,
	`hates_flavor` text
);
--> statement-breakpoint
CREATE UNIQUE INDEX `natures_name_unique` ON `natures` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `natures_slug_unique` ON `natures` (`slug`);--> statement-breakpoint
CREATE TABLE `egg_groups` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`name` text NOT NULL,
	`slug` text NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `egg_groups_name_unique` ON `egg_groups` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `egg_groups_slug_unique` ON `egg_groups` (`slug`);--> statement-breakpoint
CREATE TABLE `encounters` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`location_area_id` integer NOT NULL,
	`pokemon_id` integer NOT NULL,
	`version_id` integer NOT NULL,
	`encounter_method_id` integer NOT NULL,
	`condition_value_id` integer,
	`min_level` integer NOT NULL,
	`max_level` integer NOT NULL,
	`chance` integer NOT NULL,
	`slot` integer,
	FOREIGN KEY (`location_area_id`) REFERENCES `location_areas`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`pokemon_id`) REFERENCES `pokemon`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`version_id`) REFERENCES `versions`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`encounter_method_id`) REFERENCES `encounter_methods`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`condition_value_id`) REFERENCES `encounter_condition_values`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `pokedex_entries` (
	`pokedex_id` integer NOT NULL,
	`pokemon_id` integer NOT NULL,
	`species_id` integer NOT NULL,
	`entry_number` integer NOT NULL,
	`next_pokemon_id` integer,
	`next_species_id` integer,
	PRIMARY KEY(`pokedex_id`, `pokemon_id`),
	FOREIGN KEY (`pokedex_id`) REFERENCES `pokedexes`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`pokemon_id`) REFERENCES `pokemon`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`species_id`) REFERENCES `pokemon_species`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`next_pokemon_id`) REFERENCES `pokemon`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`next_species_id`) REFERENCES `pokemon_species`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `pokemon` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`species_id` integer NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`height` integer NOT NULL,
	`weight` integer NOT NULL,
	`hp` integer NOT NULL,
	`atk` integer NOT NULL,
	`def` integer NOT NULL,
	`spa` integer NOT NULL,
	`spd` integer NOT NULL,
	`spe` integer NOT NULL,
	`sprite_default` text,
	`sprite_shiny` text,
	`sprite_artwork` text,
	`sprite_artwork_shiny` text,
	`sprite_back_default` text,
	`sprite_back_shiny` text,
	`evolves_from_pokemon_id` integer,
	`form_of_pokemon_id` integer,
	`form_type` text,
	FOREIGN KEY (`species_id`) REFERENCES `pokemon_species`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE UNIQUE INDEX `pokemon_name_unique` ON `pokemon` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `pokemon_slug_unique` ON `pokemon` (`slug`);--> statement-breakpoint
CREATE TABLE `pokemon_abilities` (
	`pokemon_id` integer NOT NULL,
	`ability_id` integer NOT NULL,
	`slot` integer NOT NULL,
	`is_hidden` integer DEFAULT false NOT NULL,
	PRIMARY KEY(`pokemon_id`, `ability_id`),
	FOREIGN KEY (`pokemon_id`) REFERENCES `pokemon`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`ability_id`) REFERENCES `abilities`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `pokemon_evolution_details` (
	`pokemon_id` integer PRIMARY KEY NOT NULL,
	`trigger` text NOT NULL,
	`conditions` text DEFAULT '{}' NOT NULL,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	FOREIGN KEY (`pokemon_id`) REFERENCES `pokemon`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `pokemon_flavor_texts` (
	`species_id` integer NOT NULL,
	`flavor_text` text NOT NULL,
	`language_id` integer NOT NULL,
	`version_group_id` integer NOT NULL,
	PRIMARY KEY(`species_id`, `language_id`, `version_group_id`),
	FOREIGN KEY (`species_id`) REFERENCES `pokemon_species`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`version_group_id`) REFERENCES `version_groups`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `pokemon_forms` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`pokemon_id` integer NOT NULL,
	`species_id` integer NOT NULL,
	`form_name` text,
	`is_default` integer DEFAULT false NOT NULL,
	`is_battle_only` integer DEFAULT false NOT NULL,
	`is_mega` integer DEFAULT false NOT NULL,
	`order` integer,
	`sprite_default` text,
	`sprite_shiny` text,
	`sprite_artwork` text,
	`sprite_artwork_shiny` text,
	`sprite_back_default` text,
	`sprite_back_shiny` text,
	FOREIGN KEY (`pokemon_id`) REFERENCES `pokemon`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`species_id`) REFERENCES `pokemon_species`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `pokemon_genera` (
	`species_id` integer NOT NULL,
	`genus` text NOT NULL,
	`language_id` integer NOT NULL,
	PRIMARY KEY(`species_id`, `language_id`),
	FOREIGN KEY (`species_id`) REFERENCES `pokemon_species`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`language_id`) REFERENCES `languages`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `pokemon_moves` (
	`pokemon_id` integer NOT NULL,
	`move_id` integer NOT NULL,
	`learn_method_id` integer NOT NULL,
	`level` integer DEFAULT 0 NOT NULL,
	`version_group_id` integer NOT NULL,
	PRIMARY KEY(`pokemon_id`, `move_id`, `learn_method_id`, `level`, `version_group_id`),
	FOREIGN KEY (`pokemon_id`) REFERENCES `pokemon`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`move_id`) REFERENCES `moves`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`learn_method_id`) REFERENCES `move_learn_methods`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`version_group_id`) REFERENCES `version_groups`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `pokemon_species` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`pokeapi_id` integer,
	`source` text DEFAULT 'pokeapi' NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`genus` text,
	`dex_number` integer NOT NULL,
	`is_baby` integer DEFAULT false NOT NULL,
	`is_legendary` integer DEFAULT false NOT NULL,
	`is_mythical` integer DEFAULT false NOT NULL,
	`habitat` text,
	`color` text,
	`shape` text,
	`growth_rate` text,
	`base_happiness` integer,
	`capture_rate` integer,
	`gender_rate` integer,
	`hatch_counter` integer,
	`generation_id` integer,
	`order` integer,
	`dex_order` integer,
	`default_pokemon_id` integer,
	FOREIGN KEY (`generation_id`) REFERENCES `generations`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE UNIQUE INDEX `pokemon_species_name_unique` ON `pokemon_species` (`name`);--> statement-breakpoint
CREATE UNIQUE INDEX `pokemon_species_slug_unique` ON `pokemon_species` (`slug`);--> statement-breakpoint
CREATE TABLE `pokemon_stats` (
	`pokemon_id` integer NOT NULL,
	`stat_name` text NOT NULL,
	`base_stat` integer NOT NULL,
	`effort` integer DEFAULT 0 NOT NULL,
	PRIMARY KEY(`pokemon_id`, `stat_name`),
	FOREIGN KEY (`pokemon_id`) REFERENCES `pokemon`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `pokemon_types` (
	`pokemon_id` integer NOT NULL,
	`type_id` integer NOT NULL,
	`slot` integer NOT NULL,
	PRIMARY KEY(`pokemon_id`, `slot`),
	FOREIGN KEY (`pokemon_id`) REFERENCES `pokemon`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`type_id`) REFERENCES `types`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `species_egg_groups` (
	`species_id` integer NOT NULL,
	`egg_group_id` integer NOT NULL,
	PRIMARY KEY(`species_id`, `egg_group_id`),
	FOREIGN KEY (`species_id`) REFERENCES `pokemon_species`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`egg_group_id`) REFERENCES `egg_groups`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `battles` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`trainer_id_1` text NOT NULL,
	`trainer_id_2` text NOT NULL,
	`team_id_1` integer NOT NULL,
	`team_id_2` integer NOT NULL,
	`winner_id` text,
	`log` text DEFAULT '' NOT NULL,
	`played_at` integer,
	FOREIGN KEY (`trainer_id_1`) REFERENCES `trainers`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`trainer_id_2`) REFERENCES `trainers`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`team_id_1`) REFERENCES `teams`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`team_id_2`) REFERENCES `teams`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`winner_id`) REFERENCES `trainers`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `edits` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`trainer_id` text NOT NULL,
	`table_name` text NOT NULL,
	`record_id` integer NOT NULL,
	`previous_data` text,
	`new_data` text,
	`comment` text,
	`edited_at` integer,
	FOREIGN KEY (`trainer_id`) REFERENCES `trainers`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `import_log` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`entity_type` text NOT NULL,
	`pokeapi_id` integer NOT NULL,
	`action` text NOT NULL,
	`reason` text,
	`comment` text,
	`timestamp` integer
);
--> statement-breakpoint
CREATE TABLE `staged_imports` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`trainer_id` text NOT NULL,
	`entity_type` text NOT NULL,
	`pokeapi_id` integer NOT NULL,
	`payload` text,
	`status` text DEFAULT 'pending' NOT NULL,
	`error_message` text,
	`updated_at` integer,
	FOREIGN KEY (`trainer_id`) REFERENCES `trainers`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `team_members` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`team_id` integer NOT NULL,
	`slot` integer NOT NULL,
	`pokemon_id` integer NOT NULL,
	`ability_id` integer NOT NULL,
	`nature_id` integer NOT NULL,
	`held_item_id` integer,
	`move1_id` integer,
	`move2_id` integer,
	`move3_id` integer,
	`move4_id` integer,
	FOREIGN KEY (`team_id`) REFERENCES `teams`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`pokemon_id`) REFERENCES `pokemon`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`ability_id`) REFERENCES `abilities`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`nature_id`) REFERENCES `natures`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`held_item_id`) REFERENCES `items`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`move1_id`) REFERENCES `moves`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`move2_id`) REFERENCES `moves`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`move3_id`) REFERENCES `moves`(`id`) ON UPDATE no action ON DELETE no action,
	FOREIGN KEY (`move4_id`) REFERENCES `moves`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `teams` (
	`id` integer PRIMARY KEY AUTOINCREMENT NOT NULL,
	`trainer_id` text NOT NULL,
	`name` text NOT NULL,
	`created_at` integer,
	FOREIGN KEY (`trainer_id`) REFERENCES `trainers`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE TABLE `trainers` (
	`id` text PRIMARY KEY NOT NULL,
	`username` text NOT NULL,
	`password_hash` text NOT NULL,
	`role` text DEFAULT 'viewer' NOT NULL,
	`is_npc` integer DEFAULT false NOT NULL,
	`banned_at` integer,
	`created_at` integer
);
--> statement-breakpoint
CREATE UNIQUE INDEX `trainers_username_unique` ON `trainers` (`username`);--> statement-breakpoint
CREATE TABLE `sessions` (
	`id` text PRIMARY KEY NOT NULL,
	`user_id` text NOT NULL,
	`expires_at` integer NOT NULL,
	FOREIGN KEY (`user_id`) REFERENCES `trainers`(`id`) ON UPDATE no action ON DELETE cascade
);
