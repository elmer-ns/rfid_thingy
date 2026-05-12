// Hex Input
var inputs = document.querySelectorAll(".hex_code");

for (let i = 0; inputs[i]; i++) {
  let input = inputs[i];
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
    console.log("Completed!", response);
  } catch (err) {
    console.error(`Error: ${err}`);
  }
});

deactivate.addEventListener("click", async (_) => {
  try {
    const response = await fetch("./api/reader/deactivate", {
      method: "post",
    });
    console.log("Completed!", response);
  } catch (err) {
    console.error(`Error: ${err}`);
  }
});
