const history_list = document.getElementById("history_list");
const update_history_button = document.getElementById("update_history");

update_history_button.addEventListener("click", updateHistory);

async function updateHistory() {
  const response = await fetch("./api/reader/history");
  const data = await response.json();

  const nowMs = await (await fetch("./api/now")).json();

  for (let i = history_list.children.length; i < data.length; i++) {
    const element = data[i];

    const event = element.event;
    const eventMs = element.timestamp;

    deltaMs = eventMs - nowMs;
    eventDate = new Date(new Date().getTime() + deltaMs);
    eventTimestamp = `${eventDate.getHours()}:${eventDate.getMinutes()}.${eventDate.getSeconds()}`;

    let event_name = "N/A";
    let inner_fmt = "";

    if (typeof event === "string") {
      event_name = event;
    } else if (typeof event === "object") {
      const keys = Object.keys(event);
      event_name = keys[0];

      const inner = event[event_name];
      inner_fmt = JSON.stringify(inner);
    }

    console.log(event_name);
    console.log(inner_fmt);

    const div = document.createElement("div");
    div.className = "history_item";

    title = document.createElement("h3");
    content = document.createElement("p");
    timestamp = document.createElement("time");

    title.textContent = event_name;
    content.textContent = inner_fmt;
    timestamp.textContent = eventTimestamp;
    div.appendChild(title);
    div.appendChild(timestamp);
    div.appendChild(content);
    history_list.appendChild(div);
  }
}

// Status
const status_label = document.getElementById("status");

// Hex Input
var hex_inputs = document.querySelectorAll(".hex_code");

for (let i = 0; hex_inputs[i]; i++) {
  let input = hex_inputs[i];
  console.log(input);
  input.addEventListener("input", (event) => {
    input.value = input.value.replace(/[^0-9a-fA-F]+$/, "");
    input.style.height = "";
    console.log(input.scrollHeight);
    input.style.height = input.scrollHeight + "px";
  });
}

// Buttons
const activate = document.getElementById("activate");
const ddeactivate = document.getElementById("deactivate");

activate.addEventListener("click", async (_) => {
  try {
    const response = await fetch("./api/reader/activate", {
      method: "post",
    });
    status_label.innerHTML = "Active";
  } catch (err) {
    console.error(`Error: ${err}`);
  }
});

deactivate.addEventListener("click", async (_) => {
  try {
    const response = await fetch("./api/reader/deactivate", {
      method: "post",
    });
    status_label.innerHTML = "Inactive";
  } catch (err) {
    console.error(`Error: ${err}`);
  }
});
