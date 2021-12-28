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
  DOGGER: Names.Frank,
  badcop_: Names.Sarah,
  denbeigh: Names.Denbeigh,
  brimonk: Names.Brian,
  Thomas: Names.Thomas,
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
    switch (name) {
      case Names.Sarah:
        return {
          borderColor: "#4499ff",
        };
      case Names.Thomas:
        return {
          borderColor: "#33e033",
        };
      case Names.Denbeigh:
        return {
          borderColor: "#e055e0",
        };
      case Names.Brian:
        return {
          borderColor: "#ff6700",
        };
      case Names.Frank:
        return {
          borderColor: "#E0E722",
        };
    }
    return {
      borderColor: "red",
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
