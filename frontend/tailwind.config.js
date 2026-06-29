/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './src/**/*.{html,js,svelte}',
    '../templates/**/*.html',
    '../templates/**/*.jinja',
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