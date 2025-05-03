(defn search [name]
	(map (fn [c] 
		{ :name        (c :name)
		  :version     (c :default_version)
		  :description (c :description)
		  :url         (string "https://crates.io/crates/" (c :name))
		  :authors     @[(string "TODO" (c :owners))] })
		(-> (string "https://crates.io/api/v1/crates?q=" name)
			 (get-req)
			 (json->janet)
			 (get :crates)))
)

(defn info [name]
	(let [json (json->janet (get-req (string "https://crates.io/api/v1/crates/" name)))
		  crate (json :crate)
		  recent (find |(= ($ :num) (crate :default_version)) (json :versions))]
		{ :pkg { :name (crate :name)
					:version (crate :default_version)
					:description (crate :description)
					:url (string "https://crates.io/crates/" (crate :name))
					:authors @[(string "TODO" (crate :owners))] }
		  :deps @["TODO"]
		  :homepage     (crate :homepage)
		  :source       (crate :repository)
		  :groups       (tuple/join (crate :categories) (crate :keywords))
		  :downloads    (crate :downloads)
		  :license      (recent :license)
		  :release-date (recent :created_at)
		  :size         (recent :crate_size)
		}
	)
)

(defn add [pkgs]
)
