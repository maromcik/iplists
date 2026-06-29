/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    '../templates/**/*.html', // Adjust to your Minijinja templates directory
    '../templates/**/*.jinja', // For `.jinja` files if used
  ],
  safelist: [
      'text-blue-500',
  ],
  theme: {
    extend: {
      // colors: {
      //   // italianGreen: '#064221ff',
      //   // italianRed: '#901a1eff',
      //   // navbarButtons: '#1f2937',
      //   // navbarAccent: '#92400e',
      // },
      fontFamily: {
        handwriting: ['"Pacifico"', 'cursive'],
      },
    },
  },
  plugins: [],

};