# routes
Backend shall have two routes:

```
/data
/stream
```

## /data
Returns the set of all known unboxings

```json
[
    {
        "unlocker": "denbeigh",
        "key": "Chroma Key",
        "container": "Chroma Case",
        "item": {
            "origin": 8,
            "quality": 12,
            "rarity": 3,
            "a": "24028753890",
            "d": "1030953410031234813",
            "paintseed": 435,
            "defindex": 19,
            "paintindex": 776,
            "stickers": [
              {
                "sticker_id": 4965,
                "slot": 0,
                "codename": "stockh2021_team_navi_gold",
                "material": "stockh2021/navi_gold",
                "name": "Natus Vincere (Gold) | Stockholm 2021"
              },
              {
                "sticker_id": 4981,
                "slot": 1,
                "codename": "stockh2021_team_g2_gold",
                "material": "stockh2021/g2_gold",
                "name": "G2 Esports (Gold) | Stockholm 2021"
              },
              {
                "sticker_id": 1693,
                "slot": 2,
                "codename": "de_nuke_gold",
                "material": "tournament_assets/de_nuke_gold",
                "name": "Nuke (Gold)"
              },
              {
                "sticker_id": 5053,
                "slot": 3,
                "codename": "stockh2021_team_pgl_gold",
                "material": "stockh2021/pgl_gold",
                "name": "PGL (Gold) | Stockholm 2021"
              }
            ],
            "float_id": "24028753890",
            "float_value": 0.11490528285503387,
            "s": "76561198035933253",
            "m": "0",
            "image_url": "http://media.steampowered.com/apps/730/icons/econ/default_generated/weapon_p90_hy_blueprint_aqua_light_large.35f86b3da01a31539d5a592958c96356f63d1675.png",
            "min": 0,
            "max": 0.5,
            "weapon_type": "P90",
            "item_name": "Facility Negative",
            "rarity_name": "Mil-Spec Grade",
            "quality_name": "Souvenir",
            "origin_name": "Found in Crate",
            "wear_name": "Minimal Wear",
            "full_item_name": "Souvenir P90 | Facility Negative (Minimal Wear)"
        }
    }
]
```

## /steam
Opens a WebSocket that returns all new unboxings as they are received. Unboxing
events shall be sent as individual JSON blobs, e.g.:

```json
{
    "unlocker": "denbeigh",
    "key": "Chroma Key",
    "container": "Chroma Case",
    "item": {
        "origin": 8,
        "quality": 12,
        "rarity": 3,
        "a": "24028753890",
        "d": "1030953410031234813",
        "paintseed": 435,
        "defindex": 19,
        "paintindex": 776,
        "stickers": [
          {
            "sticker_id": 4965,
            "slot": 0,
            "codename": "stockh2021_team_navi_gold",
            "material": "stockh2021/navi_gold",
            "name": "Natus Vincere (Gold) | Stockholm 2021"
          },
          {
            "sticker_id": 4981,
            "slot": 1,
            "codename": "stockh2021_team_g2_gold",
            "material": "stockh2021/g2_gold",
            "name": "G2 Esports (Gold) | Stockholm 2021"
          },
          {
            "sticker_id": 1693,
            "slot": 2,
            "codename": "de_nuke_gold",
            "material": "tournament_assets/de_nuke_gold",
            "name": "Nuke (Gold)"
          },
          {
            "sticker_id": 5053,
            "slot": 3,
            "codename": "stockh2021_team_pgl_gold",
            "material": "stockh2021/pgl_gold",
            "name": "PGL (Gold) | Stockholm 2021"
          }
        ],
        "float_id": "24028753890",
        "float_value": 0.11490528285503387,
        "s": "76561198035933253",
        "m": "0",
        "image_url": "http://media.steampowered.com/apps/730/icons/econ/default_generated/weapon_p90_hy_blueprint_aqua_light_large.35f86b3da01a31539d5a592958c96356f63d1675.png",
        "min": 0,
        "max": 0.5,
        "weapon_type": "P90",
        "item_name": "Facility Negative",
        "rarity_name": "Mil-Spec Grade",
        "quality_name": "Souvenir",
        "origin_name": "Found in Crate",
        "wear_name": "Minimal Wear",
        "full_item_name": "Souvenir P90 | Facility Negative (Minimal Wear)"
    }
}
```
