const testWebsocket_FAKE_DATA = (name) => (chart, update) => {
  console.warn("FAKE DATA ENABLED");
  window.setTimeout(() => {
    update(chart, {
      key: {
        name: "Operation Riptide Case Key",
        color: null,
        image_url:
          "https://community.cloudflare.steamstatic.com/economy/image/-9a81dlWLwJ2UUGcVs_nsVtzdOEdtWwKGZZLQHTxDZ7I56KU0Zwwo4NUX4oFJZEHLbXX7gNTPcUxuxpJSXPbQv2S1MDeXkh6LBBOiej8ZQI5ivHJJDxBuY3jwdKIlaasZeyDzz0J7ZYp0rCUoo-h0FDs80c5ZW_tZNjC4FTRVLs",
      },
      case: {
        name: "Operation Riptide Case",
        color: null,
        image_url:
          "https://community.cloudflare.steamstatic.com/economy/image/-9a81dlWLwJ2UUGcVs_nsVtzdOEdtWwKGZZLQHTxDZ7I56KU0Zwwo4NUX4oFJZEHLbXU5A1PIYQNqhpOSV-fRPasw8rsUFJ5KBFZv668FFU5narKKW4SvIrhw9PZlaPwNuqAxmgBucNz2L3C8dyj31Xn-0VtMW3wdY6LMlhplna0TPI",
      },
      case_value: {
        lowest_price: 0.57,
        median_price: 0.57,
        volume: 70544,
      },
      item: {
        origin: 8,
        quality: 4,
        rarity: 5,
        a: "24410837905",
        d: "13838482118145739040",
        paint_seed: 649,
        def_index: 4,
        stickers: [],
        float_value: 0.081475824,
        s: "76561198000494793",
        m: "0",
        image_url:
          "http://media.steampowered.com/apps/730/icons/econ/default_generated/weapon_glock_cu_glock_snackattack_light_large.145d863714fb0fd6f766ef28f3639c0efded5018.png",
        min: 0.0,
        max: 1.0,
        weapon_type: "Glock-18",
        item_name: "Snack Attack",
        rarity_name: "Classified",
        quality_name: "Unique",
        origin_name: "Found in Crate",
        wear_name: "Minimal Wear",
        full_item_name: "Glock-18 | Snack Attack (Minimal Wear)",
      },
      item_value: { lowest_price: 4.52, median_price: 4.07, volume: 127 },
      at: Date.now(),
      name,
    });
  }, 2500);
  window.setTimeout(() => {
    update(chart, {
      key: {
        name: "Operation Riptide Case Key",
        color: null,
        image_url:
          "https://community.cloudflare.steamstatic.com/economy/image/-9a81dlWLwJ2UUGcVs_nsVtzdOEdtWwKGZZLQHTxDZ7I56KU0Zwwo4NUX4oFJZEHLbXX7gNTPcUxuxpJSXPbQv2S1MDeXkh6LBBOiej8ZQI5ivHJJDxBuY3jwdKIlaasZeyDzz0J7ZYp0rCUoo-h0FDs80c5ZW_tZNjC4FTRVLs",
      },
      case: {
        name: "Operation Riptide Case",
        color: null,
        image_url:
          "https://community.cloudflare.steamstatic.com/economy/image/-9a81dlWLwJ2UUGcVs_nsVtzdOEdtWwKGZZLQHTxDZ7I56KU0Zwwo4NUX4oFJZEHLbXU5A1PIYQNqhpOSV-fRPasw8rsUFJ5KBFZv668FFU5narKKW4SvIrhw9PZlaPwNuqAxmgBucNz2L3C8dyj31Xn-0VtMW3wdY6LMlhplna0TPI",
      },
      case_value: {
        lowest_price: 0.57,
        median_price: 0.57,
        volume: 70544,
      },
      item: {
        origin: 8,
        quality: 4,
        rarity: 3,
        a: "24410556996",
        d: "17216391164104167143",
        paint_seed: 102,
        def_index: 33,
        stickers: [],
        float_value: 0.3355676,
        s: "76561198000494793",
        m: "0",
        image_url:
          "http://media.steampowered.com/apps/730/icons/econ/default_generated/weapon_mp7_cu_mp7_khaki_light_large.c9fb92fece0f425328e2c5c8c536302ed2dbcf99.png",
        min: 0.0,
        max: 1.0,
        weapon_type: "MP7",
        item_name: "Guerrilla",
        rarity_name: "Mil-Spec Grade",
        quality_name: "Unique",
        origin_name: "Found in Crate",
        wear_name: "Field-Tested",
        full_item_name: "MP7 | Guerrilla (Field-Tested)",
      },
      item_value: { lowest_price: 0.08, median_price: 0.06, volume: 505 },
      at: Date.now(),
      name,
    });
  }, 10000);
  window.setTimeout(() => {
    window.setInterval(() => {
      update(chart, {
        key: {
          name: "Operation Riptide Case Key",
          color: null,
          image_url:
            "https://community.cloudflare.steamstatic.com/economy/image/-9a81dlWLwJ2UUGcVs_nsVtzdOEdtWwKGZZLQHTxDZ7I56KU0Zwwo4NUX4oFJZEHLbXX7gNTPcUxuxpJSXPbQv2S1MDeXkh6LBBOiej8ZQI5ivHJJDxBuY3jwdKIlaasZeyDzz0J7ZYp0rCUoo-h0FDs80c5ZW_tZNjC4FTRVLs",
        },
        case: {
          name: "Operation Riptide Case",
          color: null,
          image_url:
            "https://community.cloudflare.steamstatic.com/economy/image/-9a81dlWLwJ2UUGcVs_nsVtzdOEdtWwKGZZLQHTxDZ7I56KU0Zwwo4NUX4oFJZEHLbXU5A1PIYQNqhpOSV-fRPasw8rsUFJ5KBFZv668FFU5narKKW4SvIrhw9PZlaPwNuqAxmgBucNz2L3C8dyj31Xn-0VtMW3wdY6LMlhplna0TPI",
        },
        case_value: {
          lowest_price: 0.57,
          median_price: 0.57,
          volume: 70544,
        },
        item: {
          origin: 8,
          quality: 4,
          rarity: 3,
          a: "24410556996",
          d: "17216391164104167143",
          paint_seed: 102,
          def_index: 33,
          stickers: [],
          float_value: 0.3355676,
          s: "76561198000494793",
          m: "0",
          image_url:
            "http://media.steampowered.com/apps/730/icons/econ/default_generated/weapon_mp7_cu_mp7_khaki_light_large.c9fb92fece0f425328e2c5c8c536302ed2dbcf99.png",
          min: 0.0,
          max: 1.0,
          weapon_type: "MP7",
          item_name: "Guerrilla",
          rarity_name: "Mil-Spec Grade",
          quality_name: "Unique",
          origin_name: "Found in Crate",
          wear_name: "Field-Tested",
          full_item_name: "MP7 | Guerrilla (Field-Tested)",
        },
        item_value: {
          lowest_price: (Math.random() - 0.8) * 40,
          median_price: (Math.random() - 0.8) * 50,
          volume: 505,
        },
        at: Date.now(),
        name,
      });
    }, 10000);
  }, 20000);
};
