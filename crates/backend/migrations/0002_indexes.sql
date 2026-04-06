-- Índices para queries frequentes da wiki
CREATE INDEX IF NOT EXISTS idx_pokemon_dex ON pokemon(dex_number);
CREATE INDEX IF NOT EXISTS idx_pokemon_type1 ON pokemon(type1_id);
CREATE INDEX IF NOT EXISTS idx_pokemon_type2 ON pokemon(type2_id);
CREATE INDEX IF NOT EXISTS idx_learnset_pokemon ON learnset(pokemon_id);
CREATE INDEX IF NOT EXISTS idx_pokemon_abilities_pokemon ON pokemon_abilities(pokemon_id);
CREATE INDEX IF NOT EXISTS idx_pokemon_abilities_ability ON pokemon_abilities(ability_id);
CREATE INDEX IF NOT EXISTS idx_evolutions_from ON evolutions(from_id);
CREATE INDEX IF NOT EXISTS idx_forms_base ON pokemon_forms(base_id);
CREATE INDEX IF NOT EXISTS idx_type_eff_attacker ON type_effectiveness(attacker_id);
CREATE INDEX IF NOT EXISTS idx_type_eff_defender ON type_effectiveness(defender_id);
