The `classes` folder holds a file which contains the metadata format for the vault. Once you click on that file, a box icon should appear next to it. Clicking on that box will open up the table view which is how you will primarily interact withe the vault.

You shouldn't edit the actual file or the `Fileclass Fields` section after the vault is rendered, although you can change the fields to set up your own metadata structure if you are using this vault as a template.

The table view is built from [Metadata Menu](https://mdelobelle.github.io/metadatamenu/) so check their docs for any help with using it (they don't have a ton of resources but there are a few videos). You basically have the ability to show or hide specific fields, shift around their order, write Javascript filters, and do ascending or descending sorts.

For writing the Javascript filters it can be a bit weird. For me it seems to only work if I write it all on a single line, so keep that in mind. Also your code can't error, so make sure to do things like checking if your values are null before using methods on them since some files will likely render with incomplete data.

```js
// example filter for companies that include computer science majors
if (value) {return value.includes('Computer Science')} else {return false}
```

You can save your current view and access it later if you want to switch between some set of filters (I probably added a few by default so you can get an idea of how they work).

The `companies` folder contains the rendered data for all of the companies. The properties section in each file is what shows up in the table view, and the rest of each file is free for you to use how you like to take notes on each company.
