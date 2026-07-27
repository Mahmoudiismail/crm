// Notifications and Toast Handling

window.showToast = function(message) {
    // Remove existing toast if any
    const existing = document.getElementById('runner-toast');
    if (existing) {
        existing.remove();
    }

    const toastDiv = document.createElement('div');
    toastDiv.id = 'runner-toast';
    toastDiv.className = 'fixed right-4 top-4 z-50 max-w-sm rounded border border-gray-200 bg-white px-4 py-3 shadow-lg flex items-start gap-3';
    toastDiv.innerHTML = `
        <div class='flex-shrink-0'>
            <svg class="w-5 h-5 text-emerald-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" d="M17.25 6.75 22.5 12l-5.25 5.25m-10.5 0L1.5 12l5.25-5.25m7.5-3-4.5 16.5" />
            </svg>
        </div>
        <p class='text-sm font-medium text-gray-900'>${message}</p>
    `;

    document.body.appendChild(toastDiv);

    setTimeout(() => {
        const t = document.getElementById('runner-toast');
        if (t) t.remove();
    }, 4000);
};

window.redirectToDashboard = function(message) {
    if (message) {
        window.location.replace('/?toast=' + encodeURIComponent(message));
    } else {
        window.location.replace('/');
    }
};

window.addEventListener('DOMContentLoaded', function() {
    const params = new URLSearchParams(window.location.search);
    const toastMsg = params.get('toast');
    if (toastMsg) {
        window.showToast(toastMsg);

        // Remove toast from URL without refreshing the page
        const newUrl = window.location.protocol + "//" + window.location.host + window.location.pathname;
        window.history.replaceState({path:newUrl},'',newUrl);
    } else {
        // If a toast was server-rendered natively (no URL param), just set up the timeout to remove it.
        const existing = document.getElementById('runner-toast');
        if (existing) {
            setTimeout(() => {
                const t = document.getElementById('runner-toast');
                if (t) t.remove();
            }, 4000);
        }
    }
});
