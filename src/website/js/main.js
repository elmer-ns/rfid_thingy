var inputs = document.getElementsByClassName('hex_code');

for (let i=0; i<inputs.length; i++) {
    let input = inputs[i];
    console.log(input)
    input.addEventListener('input', (event) => {
        input.value = input.value.replace(/[^0-9a-fA-F]+$/, '');
    })
}