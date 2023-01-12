// Estimate the net cost (value?) of an item.
const value_estimator = ({ item_value, case_value, key }) => {
  const key_actual = key ? 2.49 : 0;
  const case_actual = case_value.lowest_price || case_value.median_price || 0;
  const item_actual = item_value.lowest_price || item_value.median_price || 0;
  return item_actual - case_actual - key_actual;
};

// Takes an array - computes the incremental sum for each element, iteratively.
const sum_over_time = (arr) => {
  let val = 0;
  const result = [];
  for (v of arr) {
    val += v;
    result.push(val);
  }
  return result;
};

const Names = {
  Sarah: "Sarah ",
  Frank: "Frank ",
  Denbeigh: "Denbeigh ",
  Brian: "Brian ",
  Thomas: "Thomas ",
};
const Players = {
  denbeigh2000: Names.Denbeigh,
  brimonk: Names.Brian,
  badcop_: Names.Sarah,
  Thomas: Names.Thomas,
  FrankDaTank159: Names.Frank,
};

const PlayerColors = {
  [Names.Denbeigh]: "#e055e0",
  [Names.Brian]: "#ff6700",
  [Names.Sarah]: "#4499ff",
  [Names.Thomas]: "#33e033",
  [Names.Frank]: "#e0e722",
};

const makeRow = (item) => {
  const image_url = item.item.image_url;
  const div = document.createElement("div");
  const textDiv = document.createElement("div");
  div.setAttribute("class", "item-row");
  textDiv.setAttribute("class", "item-info");
  const img = document.createElement("img");
  const name = document.createElement("p");
  name.setAttribute("style", `color: ${rarityToColor(item.item.rarity)}`);
  name.innerText = item.item.full_item_name || item.item.item_name || "idk";
  const unboxer = document.createElement("p");
  const unboxer_name = document.createElement("span");
  unboxer_name.innerText = item.name;
  unboxer_name.setAttribute(
    "style",
    `color: ${PlayerColors[item.name] || "red"}`
  );
  unboxer.innerText = `Unboxed by `;
  unboxer.appendChild(unboxer_name);
  const price = document.createElement("p");
  price.innerText = `$${(item.item_value.lowest_price || item.item_value.median_price || 0).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })} USD`;

  img.setAttribute("src", image_url);
  div.appendChild(img);
  textDiv.appendChild(name);
  textDiv.appendChild(unboxer);
  textDiv.appendChild(price);
  div.appendChild(textDiv);
  return div;
};

const rarityToColor = (rarity) => {
  return [
    "#AFAFAF",
    "#6496E1",
    "#4B69CD",
    "#8847FF",
    "#D32CE6",
    "#EB4B4B",
    "#CAAB05",
  ][rarity - 1];
};
// Takes a `moment()` timestamp object as `now`; returns a function that will
// determine if the given event is less than `duration` old (as of `now`).
//
// Example usage: data.filter(fresh(moment(), 1, "hours"))
// Returns: an array containing only the events that happened in the past hour.
const fresh = (now, duration, unit) => (ev) =>
  now.diff(moment(new Date(ev.at || ev.x)), unit, true) < duration;

// Zips together two arrays of the same length.
// Undefined behavior if they have different length.
const zip = (a, b) => a.map((k, i) => [k, b[i]]);

// Specifies some player-specific colors.
const getPlayerDefaults = (name) => {
  const base = (name) => {
    return {
      borderColor: PlayerColors[name] || "red",
    };
  };
  return {
    ...base(name),
    backgroundColor: "#222",
    cubicInterpolationMode: "monotone",
    tension: 0.4,
    borderWidth: 6,
    pointRadius: 7,
    // fill: {
    //   above: "#00aa0020",
    //   below: "#ff000020",
    //   target: { value: 0 },
    // },
  };
};
