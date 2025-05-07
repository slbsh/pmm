(defn parse-pkg [crate] 
	(let [owners (->> ((crate :links) :owners)
						   (string "https://crates.io")
						   (get-req) (json->janet))]
		{ :name (crate :name)
		  :version (crate :default_version)
		  :description (crate :description)
		  :url (string "https://crates.io/crates/" (crate :name))
		  :authors (map |(string ($ :login) " (" ($ :name) ")") (owners :users)) }
	)
)

(defn search [name]
	(map parse-pkg 
		(-> (string "https://crates.io/api/v1/crates?q=" name)
			 (get-req) (json->janet) (get :crates)))
)

(defn info [name]
	(let [json (-> (string "https://crates.io/api/v1/crates/" name)
						(get-req) (json->janet))
		  crate (json :crate)
		  recent (find |(= ($ :num) (crate :default_version)) (json :versions)) ]
		{ :pkg (parse-pkg crate)
		  :deps @["TODO"]
		  :homepage     (crate :homepage)
		  :source       (crate :repository)
		  :groups       (tuple/join (crate :categories) (crate :keywords))
		  :downloads    (crate :downloads)
		  :license      (recent :license)
		  :release-date (let [[date time] (string/split "T" (recent :created_at))
									 [time _]    (string/split "." time)]
							    (string date " " time)) 
		  :size         (recent :crate_size) }
	)
)

(defn add [pkgs]
	()
)
