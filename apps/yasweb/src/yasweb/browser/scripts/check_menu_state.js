new Promise(resolve => {
    let attempts = 0;
    const check = () => {
        const menu = document.querySelector('.menuModules');
        if (menu && menu.classList.contains('show-modules')) {
            resolve(true);
        } else if (attempts > 50) {
            resolve(false);
        } else {
            attempts++;
            setTimeout(check, 100);
        }
    };
    check();
})