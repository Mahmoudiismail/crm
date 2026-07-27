// Common Utilities
function debounce(func, wait) {
    let timeout;
    return function executedFunction(...args) {
        const later = () => {
            clearTimeout(timeout);
            func(...args);
        };
        clearTimeout(timeout);
        timeout = setTimeout(later, wait);
    };
}


// UI Event Listeners
document.addEventListener('DOMContentLoaded', function() {
    const mobileMenu = document.getElementById('mobile-menu');
    const openBtn = document.getElementById('open-sidebar-btn');
    const closeBtn = document.getElementById('close-sidebar-btn');
    const mobileSidebar = document.getElementById('mobile-sidebar');

    if (openBtn && mobileMenu && closeBtn && mobileSidebar) {
        openBtn.addEventListener('click', function() {
            mobileMenu.classList.remove('hidden');
            // Allow small delay for transition
            setTimeout(() => {
                mobileSidebar.classList.remove('-translate-x-full');
            }, 10);
        });

        closeBtn.addEventListener('click', function() {
            mobileSidebar.classList.add('-translate-x-full');
            setTimeout(() => {
                mobileMenu.classList.add('hidden');
            }, 300); // match duration
        });
    }
});
