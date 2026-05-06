var inputs = document.querySelectorAll('.hex_code');

for (let i = 0; inputs[i]; i++) {
    let input = inputs[i];
    console.log(input)
    input.addEventListener('input', (event) => {
        input.value = input.value.replace(/[^0-9a-fA-F]+$/, '');
        input.style.height = "";
        console.log(input.scrollHeight);
        input.style.height = input.scrollHeight + "px";
    })
}