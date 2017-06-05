# setup:
#
# install python 2.7 and pip utility
# run > pip install faker
# run > pip install mysql-connector==2.1.4
#
#
# usage:
#
# update global variables.
# run > python2 fake-data-gen.py
#
#
# package capture:
#
# start tcpdump
# run > mysql -u USER -pPASS -h DB_HOST "fake_data" -e "SELECT sku, description FROM products"


from faker import Factory
import logging
import mysql.connector
import sys


db_host = "YOUR_DB_IP_HERE"
db_port = "3306"
db_user = "testbot"
db_pass = "testbot"

database = "fake_data"
table = "products"
row_count = 100000


def main():
    faker = Factory.create()

    try:
        conn = mysql.connector.connect(user=db_user, password=db_pass, host=db_host, port=db_port)
        cursor = conn.cursor()

        create_database(cursor, database)
        create_table(cursor, table)
        populate_table(faker, cursor, database, table, row_count)

        conn.commit()

    except mysql.connector.Error as err:
        logger.error(err.msg)
        sys.exit(1)


def init_logger():
    FORMAT = '%(asctime)-15s %(message)s'
    logging.basicConfig(format=FORMAT)

    return logging.getLogger('fake-data')


def create_database(cursor, db_name):
    cursor.execute("CREATE DATABASE IF NOT EXISTS %s DEFAULT CHARACTER SET 'utf8mb4'" % db_name)
    cursor.execute("USE %s" % db_name)

    logger.warning("Database created and opened succssfully: %s" % db_name)


def create_table(cursor, table_name):
    cursor.execute(
            "CREATE TABLE IF NOT EXISTS %s ("
            "  id INT(11) AUTO_INCREMENT NOT NULL,"
            "  sku VARCHAR(25) NOT NULL,"
            "  description VARCHAR(255) NOT NULL,"
            "  PRIMARY KEY (id)"
            ") ENGINE=InnoDB" % table_name)


def populate_table(faker, cursor, db_name, table_name, count):
    cursor.execute("TRUNCATE `%s`.`%s`" % ( db_name, table_name ))
    for i in range(0, count):
        insert_row(cursor, db_name, table_name, fake_sku(faker), fake_description(faker))


def insert_row(cursor, db_name, table_name, sku, description):
    cursor.execute(
            "INSERT INTO `%s`.`%s` (sku, description)"
            "VALUES ('%s', '%s')"
            % ( db_name, table_name, sku, description ))


def fake_sku(faker):
    word = faker.md5(raw_output=False)
    return "%s-%s-%s" % ( word[0:3], word[3:5], word[5:11] )

def fake_description(faker):
    sentence = faker.sentence(nb_words=8, variable_nb_words=True)
    symbols = faker.password(length=10, special_chars=True, digits=True, upper_case=True, lower_case=True)
    return "%s | %s" % ( sentence, symbols )


logger = init_logger()

main()
