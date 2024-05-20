restcli
=======

CLI utility for easy reading and manipulate RESTful APIs

showcase
--------

With some RESTful datas like this:

```yaml
/languages/rust:
  GC: no
/languages/rust/applications/restcli:
  category: ultility
/languages/go:
  GC: yes
/languages/go/applications/etcd:
  category: database
/languages/go/applications/kubernetes:
  company: Google
/languages/C%2FC++:
  GC: no
/languages/C%2FC++/applications/linux:
  category: kernel
/languages/C%2FC++/applications/ceph:
  category: file-system
```

restcli will convert those datas into the following notation:

```
.languages
  .C/C++:
    GC no
    .applications
      .ceph:
        category file-system
      .linux:
        category kernel
  .go:
    GC yes
    .applications
      .etcd:
        category database
      .kubernetes:
        company Google
  .rust:
    GC no
    .applications.restcli:
      category ultility
```

In the future, restcli will be able to accomplish following:

1. Enter a sub level and list filtered entries
2. With pre-defined OpenAPI3 schemas loaded, restcli will be able to create,
   edit and submit entries to backend RESTful server
